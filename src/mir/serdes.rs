use crate::{
    hir,
    mir::{
        Expr, FuncD, FuncK,
        builder::{Builder, Ctx, TVar},
    },
    options::Options,
    shared::{ApiCheck, NumberKind, Range},
};

pub struct Ser {
    pub opts: Options,
    pub native: bool,
}

pub struct Des {
    pub opts: Options,
    pub native: bool,
    pub check: bool,
}

pub trait Serdes {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser, Self> + 'ty;

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des, Self> + 'ty;
}

impl Ser {
    fn check_type(&self, b: &mut Builder, expr: Expr, ty: &'static str) {
        if matches!(self.opts.apicheck, ApiCheck::Full) {
            b.assert(
                Expr::Global("type").call(vec![expr]).eq(ty),
                format!("value is not of type {ty}"),
            )
        }
    }

    fn check_range(&self, b: &mut Builder, expr: Expr, range: Range) {
        if matches!(self.opts.apicheck, ApiCheck::Full | ApiCheck::Some) {
            match (range.min, range.max) {
                (Some(min), Some(max)) => {
                    let msg = format!("value out of bounds [{min}, {max}]");
                    b.assert(expr.clone().ge(min).and(expr.le(max)), msg);
                }

                (Some(min), None) => {
                    let msg = format!("value out of bounds [{min}, inf]");
                    b.assert(expr.ge(min), msg);
                }

                (None, Some(max)) => {
                    let msg = format!("value out of bounds [-inf, {max}]");
                    b.assert(expr.le(max), msg);
                }

                (None, None) => {}
            }
        }
    }

    fn length(&self, b: &mut Builder, ctx: &Ctx, len: hir::Length, expr: Expr) -> TVar {
        if let Some(exact) = len.exact() {
            b.init(exact)
        } else {
            let var = b.init(expr);
            self.check_range(b, var.expr(), len.into());

            let kind = len.kind();
            b.check(ctx, kind.size());
            b.write_k(ctx, kind.into(), &var);

            var
        }
    }
}

impl Des {
    fn check_range(&self, b: &mut Builder, expr: Expr, range: Range) {
        if self.check {
            match (range.min, range.max) {
                (Some(min), Some(max)) => {
                    let msg = format!("value out of bounds [{min}, {max}]");
                    b.assert(expr.clone().ge(min).and(expr.le(max)), msg);
                }

                (Some(min), None) => {
                    let msg = format!("value out of bounds [{min}, inf]");
                    b.assert(expr.ge(min), msg);
                }

                (None, Some(max)) => {
                    let msg = format!("value out of bounds [-inf, {max}]");
                    b.assert(expr.le(max), msg);
                }

                (None, None) => {}
            }
        }
    }

    fn length(&self, b: &mut Builder, ctx: &Ctx, len: hir::Length) -> TVar {
        if let Some(exact) = len.exact() {
            b.init(exact)
        } else {
            let var = b.read_k(ctx, len.kind().into());
            self.check_range(b, var.expr(), len.into());

            var
        }
    }
}

impl Serdes for hir::Type {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        #[allow(clippy::type_complexity)]
        let cb: Box<dyn FnOnce(&mut Builder, &Ctx, Expr)> = match self {
            hir::Type::Boolean(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Number(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Vector(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::BinaryString(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Utf8String(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Array(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Set(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Map(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Enum(ty) => Box::new(ty.ser(b, ser)),
            hir::Type::Struct(ty) => Box::new(ty.ser(b, ser)),
        };

        move |b, ctx, from| cb(b, ctx, from)
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        #[allow(clippy::type_complexity)]
        let cb: Box<dyn FnOnce(&mut Builder, &Ctx) -> TVar> = match self {
            hir::Type::Boolean(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Number(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Vector(ty) => Box::new(ty.des(b, ser)),
            hir::Type::BinaryString(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Utf8String(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Array(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Set(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Map(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Enum(ty) => Box::new(ty.des(b, ser)),
            hir::Type::Struct(ty) => Box::new(ty.des(b, ser)),
        };

        move |b, ctx| cb(b, ctx)
    }
}

impl Serdes for hir::BooleanType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "boolean");

            let value = Expr::Global("bool").index(from);
            b.check(ctx, 1);
            b.write_k(ctx, FuncK::U8, value);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        move |b: &mut Builder, ctx: &Ctx| {
            let value = b.read_k(ctx, FuncK::U8);
            b.init(Expr::Global("bool").index(&value))
        }
    }
}

impl Serdes for hir::NumberType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "number");
            ser.check_range(b, from.clone(), self.range);

            b.check(ctx, self.kind.size());
            b.write_k(ctx, FuncK::from(self.kind), from);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        move |b: &mut Builder, ctx: &Ctx| {
            let value = b.read_k(ctx, self.kind.into());
            des.check_range(b, value.expr(), self.range);

            if des.check && !matches!(self.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                b.assert(value.expr().eq(&value), "value is nan");
            }

            if matches!(self.kind, NumberKind::U24 | NumberKind::I24) {
                if des.native {
                    b.init(Expr::Global("bit32.band").call(vec![value.expr(), 0x00FFFFFF.into()]))
                } else {
                    b.init(value.expr().mud(256 * 256 * 256))
                }
            } else {
                value
            }
        }
    }
}

impl Serdes for hir::VectorType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let x = self.x.ser(b, ser);
        let y = self.y.ser(b, ser);
        let z = self.z.as_ref().map(|z| z.ser(b, ser));

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "vector");

            x(b, ctx, from.clone().index("x"));
            y(b, ctx, from.clone().index("y"));

            if let Some(z) = z {
                z(b, ctx, from.clone().index("z"));
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let x = self.x.des(b, des);
        let y = self.y.des(b, des);
        let z = self.z.as_ref().map(|z| z.des(b, des));

        move |b: &mut Builder, ctx: &Ctx| {
            let x = x(b, ctx);
            let y = y(b, ctx);

            if let Some(z) = z {
                let z = z(b, ctx);

                b.init(Expr::vector(&x, &y, &z))
            } else {
                b.init(Expr::vector(&x, &y, 0))
            }
        }
    }
}

impl Serdes for hir::BinaryStringType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "string");

            let len = ser.length(b, ctx, self.len, from.clone().len());

            b.check(ctx, &len);
            b.write_d(ctx, FuncD::String, from, &len);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        move |b: &mut Builder, ctx: &Ctx| {
            let len = des.length(b, ctx, self.len);
            b.read_d(ctx, FuncD::String, &len)
        }
    }
}

impl Serdes for hir::Utf8StringType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "string");

            if matches!(ser.opts.apicheck, ApiCheck::Full | ApiCheck::Some) {
                b.assert(
                    Expr::Global("utf8.len").call(vec![from.clone()]),
                    "string is not valid utf8",
                );
            }

            let len = ser.length(b, ctx, self.len, from.clone().len());

            b.check(ctx, &len);
            b.write_d(ctx, FuncD::String, from, &len);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        move |b: &mut Builder, ctx: &Ctx| {
            let len = des.length(b, ctx, self.len);
            let str = b.read_d(ctx, FuncD::String, &len);

            if des.check {
                b.assert(
                    Expr::Global("utf8.len").call(vec![str.expr()]),
                    "string is not valid utf8",
                );
            }

            str
        }
    }
}

impl Serdes for hir::ArrayType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let item = self.item.ser(b, ser);

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "table");

            let len = ser.length(b, ctx, self.len, from.clone().len());
            b.for_range(1, &len, |b, i| {
                let value = b.init(from.clone().index(i.expr()));
                item(b, ctx, value.expr());
            });
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let item = self.item.des(b, des);

        move |b: &mut Builder, ctx: &Ctx| {
            let len = des.length(b, ctx, self.len);
            let arr = b.init(Expr::array(&len));

            b.for_range(1, &len, |b, i| {
                let value = item(b, ctx);
                b.assign_index(&arr, i.expr(), value.expr());
            });

            arr
        }
    }
}

impl Serdes for hir::SetType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let item = self.item.ser(b, ser);

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "table");

            b.check(ctx, self.len.kind().size());
            let len_pos = b.reserve(ctx, self.len.kind().size());
            let len_var = b.init(0);

            b.for_table(from, |b, i, _| {
                b.assign(&len_var, len_var.expr().add(1));
                item(b, ctx, i.expr());
            });

            ser.check_range(b, len_var.expr(), self.len.into());
            b.write_reserved_k(ctx, self.len.kind().into(), &len_var);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let item = self.item.des(b, des);

        move |b: &mut Builder, ctx: &Ctx| {
            let len = b.read_k(ctx, self.len.kind().into());
            des.check_range(b, len.expr(), self.len.into());

            let set = b.init(Expr::Table(vec![]));
            b.for_range(1, &len, |b, _| {
                let value = item(b, ctx);
                b.assign_index(&set, &value, true);
            });

            set
        }
    }
}

impl Serdes for hir::MapType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let index = self.index.ser(b, ser);
        let value = self.value.ser(b, ser);

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "table");

            b.check(ctx, self.len.kind().size());
            let len_pos = b.reserve(ctx, self.len.kind().size());
            let len_var = b.init(0);

            b.for_table(from.clone(), |b, i, v| {
                b.assign(&len_var, len_var.expr().add(1));
                index(b, ctx, i.expr());
                value(b, ctx, v.expr());
            });

            ser.check_range(b, len_var.expr(), self.len.into());
            b.write_reserved_k(ctx, self.len.kind().into(), &len_var);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let index = self.index.des(b, des);
        let value = self.value.des(b, des);

        move |b: &mut Builder, ctx: &Ctx| {
            let len = b.read_k(ctx, self.len.kind().into());
            des.check_range(b, len.expr(), self.len.into());

            let map = b.init(Expr::Table(vec![]));
            b.for_range(1, &len, |b, _| {
                let i = index(b, ctx);
                let v = value(b, ctx);
                b.assign_index(&map, i.expr(), v.expr());
            });

            map
        }
    }
}

impl Serdes for hir::EnumType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let map = b.init(Expr::Table(
            self.variants
                .iter()
                .enumerate()
                .map(|(i, v)| (Expr::from(v.as_str()), Expr::from(i as u32 + 1)))
                .collect(),
        ));

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "string");

            let value = b.init(map.expr().index(from));

            if matches!(ser.opts.apicheck, ApiCheck::Full) {
                b.assert(value.expr(), "enum value is not a valid variant");
            }

            b.check(ctx, self.number.kind.size());
            b.write_k(ctx, FuncK::from(self.number.kind), &value);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let map = b.init(Expr::Table(
            self.variants
                .iter()
                .enumerate()
                .map(|(i, v)| (Expr::from(i as u32 + 1), Expr::from(v.as_str())))
                .collect(),
        ));

        move |b: &mut Builder, ctx: &Ctx| {
            let value = b.read_k(ctx, self.number.kind.into());
            des.check_range(
                b,
                value.expr(),
                Range {
                    min: Some(1.0),
                    max: Some(self.variants.len() as f64),
                },
            );

            b.init(map.expr().index(value.expr()))
        }
    }
}

impl Serdes for hir::StructType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl FnOnce(&mut Builder, &Ctx, Expr) + use<'ty, 'ser> + 'ty {
        let fields = self
            .fields
            .iter()
            .map(|(name, ty)| (name.as_str(), ty.ser(b, ser)))
            .collect::<Vec<_>>();

        move |b: &mut Builder, ctx: &Ctx, from: Expr| {
            ser.check_type(b, from.clone(), "table");

            for (name, ser) in fields.into_iter() {
                let value = b.init(from.clone().index(name));
                ser(b, ctx, value.expr());
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl FnOnce(&mut Builder, &Ctx) -> TVar + use<'ty, 'des> + 'ty {
        let fields = self
            .fields
            .iter()
            .map(|(name, ty)| (name.as_str(), ty.des(b, des)))
            .collect::<Vec<_>>();

        move |b: &mut Builder, ctx: &Ctx| {
            let mut vars = Vec::new();

            for (name, des) in fields.into_iter() {
                let value = des(b, ctx);
                vars.push((name, value));
            }

            b.init(Expr::Table(
                vars.iter()
                    .map(|(name, var)| (Expr::from(*name), var.expr()))
                    .collect(),
            ))
        }
    }
}
