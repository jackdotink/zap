use crate::{
    builder::{Builder, InitVar},
    ir::{Expr, FuncD},
    types::{NumberKind, Range, Type},
};

fn check_type(b: &mut Builder, expr: impl Into<Expr>, ty: &'static str) {
    b.assert(expr.into().ty().eq(ty), format!("expected {ty}"));
}

fn check_range(b: &mut Builder, expr: impl Into<Expr>, range: &Range) {
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
    b.assert(expr.into().utf8(), "not a valid utf8 string");
}

#[derive(Debug, Clone, Copy)]
pub enum ApiCheck {
    None,
    Some,
    Full,
}

#[derive(Debug, Clone, Copy)]
pub struct Ser {
    pub apicheck: ApiCheck,
}

macro_rules! apicheck_some {
    ($b:expr, $block:block) => {
        if matches!($b.apicheck, ApiCheck::Some | ApiCheck::Full) {
            $block
        }
    };

    ($b:expr, $stmt:stmt) => {
        if matches!($b.apicheck, ApiCheck::Some | ApiCheck::Full) {
            $stmt
        }
    };
}

macro_rules! apicheck_full {
    ($s:expr, $block:block) => {
        if matches!($s.apicheck, ApiCheck::Full) {
            $block
        }
    };

    ($s:expr, $stmt:stmt) => {
        if matches!($s.apicheck, ApiCheck::Full) {
            $stmt
        }
    };
}

pub fn ser(s: Ser, b: &mut Builder, ty: Type, from: Expr) {
    match ty {
        Type::Number(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "number"));
            apicheck_some!(s, check_range(b, from.clone(), &ty.range));

            if !matches!(ty.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                apicheck_some!(s, b.assert(from.clone().eq(from.clone()), "value is nan"))
            }

            b.alloc_k(ty.kind.size());
            b.write_k(ty.kind, from);
        }

        Type::Vector(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "vector"));

            ser(s, b, Type::Number(ty.x), from.clone().index("x"));
            ser(s, b, Type::Number(ty.y), from.clone().index("y"));

            if let Some(z) = ty.z {
                ser(s, b, Type::Number(z), from.clone().index("z"));
            }
        }

        Type::BinaryString(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "string"));

            if let Some(len) = ty.len.exact() {
                apicheck_some!(s, check_range(b, from.clone().len(), &ty.len));

                b.alloc_k(len as u32);
                b.write_d(FuncD::String, from, len);
            } else {
                let len = b.expr(from.clone().len());
                apicheck_some!(s, check_range(b, &len, &ty.len));

                b.alloc_k(ty.len.len_kind().size());
                b.write_k(ty.len.len_kind(), &len);
                b.alloc_d(&len);
                b.write_d(FuncD::String, from, &len);
            }
        }

        Type::Utf8String(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "string"));
            apicheck_some!(s, check_utf8(b, from.clone()));

            if let Some(len) = ty.len.exact() {
                apicheck_some!(s, check_range(b, from.clone().len(), &ty.len));

                b.alloc_k(len as u32);
                b.write_d(FuncD::String, from, len);
            } else {
                let len = b.expr(from.clone().len());
                apicheck_some!(s, check_range(b, &len, &ty.len));

                b.alloc_k(ty.len.len_kind().size());
                b.write_k(ty.len.len_kind(), &len);
                b.alloc_d(&len);
                b.write_d(FuncD::String, from, &len);
            }
        }

        Type::Array(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "table"));

            if let Some(len) = ty.len.exact() {
                apicheck_some!(s, check_range(b, from.clone().len(), &ty.len));

                b.iter_range(1f64, len, |b, index| {
                    let item = b.expr(from.clone().index(index));
                    ser(s, b, *ty.item, Expr::from(&item));
                });
            } else {
                let len = b.expr(from.clone().len());
                apicheck_some!(s, check_range(b, &len, &ty.len));

                b.alloc_k(ty.len.len_kind().size());
                b.write_k(ty.len.len_kind(), &len);

                b.iter_range(1f64, &len, |b, index| {
                    let item = b.expr(from.clone().index(index));
                    ser(s, b, *ty.item, Expr::from(&item));
                });
            }
        }

        Type::Set(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "table"));

            let loc = b.reserve_k(ty.len.len_kind().size());
            let len = b.expr(0f64);

            b.iter_map(from, |b, value, _| {
                b.assign(&len, Expr::from(&len).add(1f64));
                ser(s, b, *ty.item, Expr::from(value));
            });

            apicheck_some!(s, check_range(b, &len, &ty.len));
            b.write_reserved_k(ty.len.len_kind(), &loc, &len);
        }

        Type::Map(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "table"));

            let loc = b.reserve_k(ty.len.len_kind().size());
            let len = b.expr(0f64);

            b.iter_map(from, |b, index, value| {
                b.assign(&len, Expr::from(&len).add(1f64));
                ser(s, b, *ty.index, Expr::from(index));
                ser(s, b, *ty.value, Expr::from(value));
            });

            apicheck_some!(s, check_range(b, &len, &ty.len));
            b.write_reserved_k(ty.len.len_kind(), &loc, &len);
        }

        Type::Struct(ty) => {
            apicheck_full!(s, check_type(b, from.clone(), "table"));

            for (name, ty) in &ty.fields {
                let field = b.expr(from.clone().index(name.as_str()));
                ser(s, b, ty.clone(), Expr::from(&field));
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Des {
    pub check: bool,
}

macro_rules! check {
    ($b:expr, $block:block) => {
        if $b.check {
            $block
        }
    };

    ($b:expr, $stmt:stmt) => {
        if $b.check {
            $stmt
        }
    };
}

pub fn des(d: Des, b: &mut Builder, ty: Type) -> InitVar {
    match ty {
        Type::Number(ty) => {
            let value = b.read_k(ty.kind);
            check!(d, check_range(b, &value, &ty.range));

            if !matches!(ty.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                check!(d, b.assert(Expr::from(&value).eq(&value), "value is nan"))
            }

            value
        }

        Type::Vector(ty) => {
            let x = des(d, b, Type::Number(ty.x));
            let y = des(d, b, Type::Number(ty.y));
            let z = ty.z.map(|z| des(d, b, Type::Number(z)));

            b.expr(Expr::Vector(
                Box::new(Expr::from(&x)),
                Box::new(Expr::from(&y)),
                Box::new(z.map(|z| Expr::from(&z)).unwrap_or(Expr::from(0f64))),
            ))
        }

        Type::BinaryString(ty) => {
            if let Some(len) = ty.len.exact() {
                b.read_d(FuncD::String, len)
            } else {
                let len = b.read_k(ty.len.len_kind());
                check!(d, check_range(b, &len, &ty.len));

                b.read_d(FuncD::String, &len)
            }
        }

        Type::Utf8String(ty) => {
            if let Some(len) = ty.len.exact() {
                let str = b.read_d(FuncD::String, len);
                check!(d, check_utf8(b, &str));

                str
            } else {
                let len = b.read_k(ty.len.len_kind());
                check!(d, check_range(b, &len, &ty.len));

                let str = b.read_d(FuncD::String, &len);
                check!(d, check_utf8(b, &str));

                str
            }
        }

        Type::Array(ty) => {
            if let Some(len) = ty.len.exact() {
                let arr = b.expr(Expr::Array(Box::new(Expr::from(len))));
                b.iter_range(1f64, len, |b, index| {
                    let value = des(d, b, *ty.item);
                    b.assign_index(&arr, index, &value);
                });

                arr
            } else {
                let len = b.read_k(ty.len.len_kind());
                check!(d, check_range(b, &len, &ty.len));

                let arr = b.expr(Expr::Array(Box::new(Expr::from(&len))));
                b.iter_range(1f64, &len, |b, index| {
                    let value = des(d, b, *ty.item);
                    b.assign_index(&arr, index, &value);
                });

                arr
            }
        }

        Type::Set(ty) => {
            let len = b.read_k(ty.len.len_kind());
            check!(d, check_range(b, &len, &ty.len));

            let set = b.expr(Expr::Table);
            b.iter_range(1f64, &len, |b, _| {
                let value = des(d, b, *ty.item);
                b.assign_index(&set, &value, Expr::Boolean(true));
            });

            set
        }

        Type::Map(ty) => {
            let len = b.read_k(ty.len.len_kind());
            check!(d, check_range(b, &len, &ty.len));

            let map = b.expr(Expr::Table);
            b.iter_range(1f64, &len, |b, _| {
                let key = des(d, b, *ty.index);
                let value = des(d, b, *ty.value);
                b.assign_index(&map, &key, &value);
            });

            map
        }

        Type::Struct(ty) => {
            let mut fields = Vec::new();

            for (name, ty) in &ty.fields {
                let field = des(d, b, ty.clone());
                fields.push((name.clone(), field));
            }

            b.expr(Expr::Struct(
                fields
                    .iter()
                    .map(|(name, var)| (name.clone(), Expr::from(var)))
                    .collect(),
            ))
        }
    }
}
