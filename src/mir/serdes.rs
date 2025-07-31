use std::rc::Rc;

use crate::{
    hir,
    mir::{
        Expr, FuncD, FuncK,
        builder::{Builder, InitVar},
    },
    shared::{ApiCheck, NumberKind, Options, Range},
};

fn check_type(b: &mut Builder, expr: impl Into<Expr>, ty: &'static str) {
    b.assert(
        Expr::Type(Box::new(expr.into())).eq(ty),
        format!("expected {ty}"),
    );
}

fn check_range(b: &mut Builder, expr: impl Into<Expr>, range: Range) {
    let expr = expr.into();

    if let Some(exact) = range.exact() {
        b.assert(expr.clone().eq(exact), format!("not equal to {exact}"));
    } else {
        match (range.min, range.max) {
            (Some(min), Some(max)) => b.assert(
                expr.clone().ge(min).and(expr.clone().le(max)),
                format!("value out of range [{min}, {max}]"),
            ),

            (Some(min), None) => b.assert(expr.clone().ge(min), format!("value less than {min}")),

            (None, Some(max)) => {
                b.assert(expr.clone().le(max), format!("value greater than {max}"))
            }

            (None, None) => {}
        }
    }
}

fn check_utf8(b: &mut Builder, expr: impl Into<Expr>) {
    b.assert(Expr::Utf8(Box::new(expr.into())), "not a valid utf8 string");
}

#[derive(Clone)]
pub struct Ser {
    pub options: Rc<Options>,
    pub native: bool,
}

#[derive(Clone)]
pub struct Des {
    pub options: Rc<Options>,
    pub native: bool,
    pub check: bool,
}

macro_rules! apicheck_some {
    ($serdes:expr, $block:block) => {
        if matches!($serdes.options.apicheck(), ApiCheck::Some | ApiCheck::Full) {
            $block
        }
    };

    ($serdes:expr, $stmt:stmt) => {
        if matches!($serdes.options.apicheck(), ApiCheck::Some | ApiCheck::Full) {
            $stmt
        }
    };
}

macro_rules! apicheck_full {
    ($serdes:expr, $block:block) => {
        if matches!($serdes.options.apicheck(), ApiCheck::Full) {
            $block
        }
    };

    ($serdes:expr, $stmt:stmt) => {
        if matches!($serdes.options.apicheck(), ApiCheck::Full) {
            $stmt
        }
    };
}

macro_rules! check {
    ($serdes:expr, $block:block) => {
        if $serdes.check {
            $block
        }
    };

    ($serdes:expr, $stmt:stmt) => {
        if $serdes.check {
            $stmt
        }
    };
}

pub trait Serdes {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser, Self> + 'ty;

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des, Self> + 'ty;
}

impl Serdes for hir::Type {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        #[allow(clippy::type_complexity)]
        let cb: Box<dyn Fn(&mut Builder, Expr)> = match self {
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

        move |b: &mut Builder, from: Expr| {
            cb(b, from);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> + 'ty {
        let cb: Box<dyn Fn(&mut Builder) -> InitVar> = match self {
            hir::Type::Boolean(ty) => Box::new(ty.des(b, des)),
            hir::Type::Number(ty) => Box::new(ty.des(b, des)),
            hir::Type::Vector(ty) => Box::new(ty.des(b, des)),
            hir::Type::BinaryString(ty) => Box::new(ty.des(b, des)),
            hir::Type::Utf8String(ty) => Box::new(ty.des(b, des)),
            hir::Type::Array(ty) => Box::new(ty.des(b, des)),
            hir::Type::Set(ty) => Box::new(ty.des(b, des)),
            hir::Type::Map(ty) => Box::new(ty.des(b, des)),
            hir::Type::Enum(ty) => Box::new(ty.des(b, des)),
            hir::Type::Struct(ty) => Box::new(ty.des(b, des)),
        };

        move |b: &mut Builder| cb(b)
    }
}

impl Serdes for hir::BooleanType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "boolean"));

            b.alloc_k(1);
            b.write_k(FuncK::U8, from.bit());
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        _: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> + 'ty {
        move |b: &mut Builder| {
            let value = b.read_k(FuncK::U8);
            b.expr(value.expr().eq(1))
        }
    }
}

impl Serdes for hir::NumberType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "number"));
            apicheck_some!(ser, check_range(b, from.clone(), self.range));

            b.alloc_k(self.kind.size());
            b.write_k(self.kind, from);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        move |b: &mut Builder| {
            let value = b.read_k(self.kind);
            check!(des, check_range(b, &value, self.range));

            if !matches!(self.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                check!(des, b.assert(value.expr().eq(&value), "value is nan"));
            }

            if matches!(self.kind, NumberKind::U24 | NumberKind::I24) {
                if des.native {
                    b.expr(value.expr().band(0x00FFFFFF))
                } else {
                    b.expr(value.expr().mud(256 * 256 * 256))
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
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        let x = self.x.ser(b, ser);
        let y = self.y.ser(b, ser);
        let z = self.z.as_ref().map(|z| z.ser(b, ser));

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "vector"));

            x(b, from.clone().index("x"));
            y(b, from.clone().index("y"));

            if let Some(z) = &z {
                z(b, from.clone().index("z"));
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        let x = self.x.des(b, des);
        let y = self.y.des(b, des);
        let z = self.z.as_ref().map(|z| z.des(b, des));

        move |b: &mut Builder| {
            let x = x(b);
            let y = y(b);
            let z = z.as_ref().map(|z| z(b));

            b.expr(Expr::Vector(
                Box::new(x.expr()),
                Box::new(y.expr()),
                Box::new(z.map_or(Expr::Number(0f64), |z| z.expr())),
            ))
        }
    }
}

impl Serdes for hir::BinaryStringType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "string"));

            let len = self.len.ser_obj(b, ser, from.clone());
            if let Some(exact) = self.len.exact() {
                b.alloc_k(exact);
                b.write_d(FuncD::String, from, exact);
            } else {
                b.alloc_d(&len);
                b.write_d(FuncD::String, from, &len);
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        move |b: &mut Builder| {
            let len = self.len.des(b, des);
            b.read_d(FuncD::String, &len)
        }
    }
}

impl Serdes for hir::Utf8StringType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "string"));
            apicheck_some!(ser, check_utf8(b, from.clone()));

            let len = self.len.ser_obj(b, ser, from.clone());
            if let Some(exact) = self.len.exact() {
                b.alloc_k(exact);
                b.write_d(FuncD::String, from, exact);
            } else {
                b.alloc_d(&len);
                b.write_d(FuncD::String, from, &len);
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        _: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        move |b: &mut Builder| {
            let len = self.len.des(b, des);
            let str = b.read_d(FuncD::String, &len);
            check!(des, check_utf8(b, &str));

            str
        }
    }
}

impl Serdes for hir::ArrayType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        let item = self.item.ser(b, ser);

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "table"));
            let len = self.len.ser_obj(b, ser, from.clone());

            b.for_range(1, &len, |b, i| {
                let value = b.expr(from.clone().index(i));
                item(b, value.expr());
            });
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        let item = self.item.des(b, des);

        move |b: &mut Builder| {
            let len = self.len.des(b, des);
            let tbl = b.expr(if let Some(exact) = self.len.exact() {
                Expr::Array(Box::new(Expr::Number(exact as f64)))
            } else {
                Expr::Array(Box::new(len.expr()))
            });

            b.for_range(1, &len, |b, i| {
                let value = item(b);
                b.assign_index(&tbl, i, &value);
            });

            tbl
        }
    }
}

impl Serdes for hir::SetType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        let item = self.item.ser(b, ser);

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "table"));

            b.alloc_k(self.len.kind().size());
            let loc = b.reserve_k(self.len.kind().size());
            let len = b.expr(0);

            b.for_table(from, |b, index, _| {
                item(b, index.expr());
                b.assign(&len, len.expr().add(1));
            });

            self.len.ser_check(b, ser, &len);
            b.write_reserved_k(self.len.kind(), &loc, &len);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        let item = self.item.des(b, des);

        move |b: &mut Builder| {
            let len = self.len.des(b, des);
            let tbl = b.expr(Expr::Table(vec![]));

            b.for_range(1, &len, |b, _| {
                let value = item(b);
                b.assign_index(&tbl, &value, Expr::Boolean(true));
            });

            tbl
        }
    }
}

impl Serdes for hir::MapType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        let index = self.index.ser(b, ser);
        let value = self.value.ser(b, ser);

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "table"));

            b.alloc_k(self.len.kind().size());
            let loc = b.reserve_k(self.len.kind().size());
            let len = b.expr(0);

            b.for_table(from, |b, key, val| {
                index(b, key.expr());
                value(b, val.expr());
                b.assign(&len, len.expr().add(1));
            });

            self.len.ser_check(b, ser, &len);
            b.write_reserved_k(self.len.kind(), &loc, &len);
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        let index = self.index.des(b, des);
        let value = self.value.des(b, des);

        move |b: &mut Builder| {
            let len = self.len.des(b, des);
            let tbl = b.expr(Expr::Table(vec![]));

            b.for_range(1, &len, |b, _| {
                let key = index(b);
                let val = value(b);
                b.assign_index(&tbl, &key, &val);
            });

            tbl
        }
    }
}

impl Serdes for hir::EnumType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> + 'ty {
        let variants = b.expr(Expr::Table(
            self.variants
                .iter()
                .enumerate()
                .map(|(i, v)| (Expr::from(v.as_str()), Expr::from(i as f64)))
                .collect(),
        ));

        let number = self.number.ser(b, ser);

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "string"));

            let value = b.expr(variants.expr().index(from));
            apicheck_some!(ser, b.assert(value.expr(), "not a valid enum variant"));

            number(b, value.expr());
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> + 'ty {
        let variants = b.expr(Expr::Table(
            self.variants
                .iter()
                .enumerate()
                .map(|(i, v)| (Expr::from(i as f64), Expr::from(v.as_str())))
                .collect(),
        ));

        let number = self.number.des(b, des);

        move |b: &mut Builder| {
            let value = number(b);
            b.expr(variants.expr().index(&value))
        }
    }
}

impl Serdes for hir::StructType {
    fn ser<'ty, 'b, 'ser: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        ser: &'ser Ser,
    ) -> impl Fn(&mut Builder, Expr) + use<'ty, 'ser> {
        let fields = self
            .fields
            .iter()
            .map(|(name, ty)| (name.as_str(), ty.ser(b, ser)))
            .collect::<Vec<_>>();

        move |b: &mut Builder, from: Expr| {
            apicheck_full!(ser, check_type(b, from.clone(), "table"));

            for (name, ser) in &fields {
                let value = b.expr(from.clone().index(*name));
                ser(b, value.expr());
            }
        }
    }

    fn des<'ty, 'b, 'des: 'ty>(
        &'ty self,
        b: &'b mut Builder,
        des: &'des Des,
    ) -> impl Fn(&mut Builder) -> InitVar + use<'ty, 'des> {
        let fields = self
            .fields
            .iter()
            .map(|(name, ty)| (name.as_str(), ty.des(b, des)))
            .collect::<Vec<_>>();

        move |b: &mut Builder| {
            let mut values = Vec::new();

            for (name, des) in &fields {
                values.push((name.to_string(), des(b)));
            }

            b.expr(Expr::Table(
                values
                    .iter()
                    .map(|(name, value)| (Expr::from(name.as_str()), Expr::from(value)))
                    .collect(),
            ))
        }
    }
}

impl hir::Length {
    fn ser_check(&self, b: &mut Builder, ser: &Ser, len: impl Into<Expr>) {
        if let Some(exact) = self.exact() {
            apicheck_some!(ser, b.assert(len.into().eq(exact), "bad length"));
        } else {
            apicheck_some!(ser, check_range(b, len.into(), Range::from(*self)));
        }
    }

    fn ser_len(&self, b: &mut Builder, ser: &Ser, len: impl Into<Expr>) {
        if let Some(exact) = self.exact() {
            self.ser_check(b, ser, exact);
        } else {
            let len = len.into();
            apicheck_some!(ser, check_range(b, len.clone(), Range::from(*self)));

            b.alloc_k(self.kind().size());
            b.write_k(self.kind(), len);
        }
    }

    fn ser_obj(&self, b: &mut Builder, ser: &Ser, obj: Expr) -> InitVar {
        if let Some(exact) = self.exact() {
            self.ser_len(b, ser, obj.len());
            b.expr(exact)
        } else {
            let len = b.expr(obj.len());
            self.ser_len(b, ser, &len);

            len
        }
    }

    fn des(&self, b: &mut Builder, des: &Des) -> InitVar {
        if let Some(exact) = self.exact() {
            b.expr(exact)
        } else {
            let len = b.read_k(self.kind());
            check!(des, check_range(b, &len, Range::from(*self)));

            len
        }
    }
}
