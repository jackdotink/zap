use std::{cell::RefCell, rc::Rc};

use crate::{
    ir::{Block, Expr, FuncD, FuncK, Instr, Var},
    types::{NumberKind, Range, Type},
};

pub fn build(ty: Type, checks: Check) -> Block {
    let mut builder = Builder::new(checks);
    Builder::ser(&mut builder, ty, Expr::Root);

    Block {
        instrs: builder.instrs,
    }
}

#[derive(Default)]
struct RegistryInner {
    vars: u16,
    free: Vec<u16>,
}

#[derive(Default, Clone)]
struct Registry {
    inner: Rc<RefCell<RegistryInner>>,
}

impl Registry {
    fn var(&self) -> UninitVar {
        let mut inner = self.inner.borrow_mut();

        if let Some(var) = inner.free.pop() {
            UninitVar {
                var,
                reg: self.clone(),
            }
        } else {
            let var = inner.vars;
            inner.vars += 1;
            UninitVar {
                var,
                reg: self.clone(),
            }
        }
    }

    fn free(&self, var: u16) {
        self.inner.borrow_mut().free.push(var);
    }
}

#[must_use]
struct UninitVar {
    var: u16,
    reg: Registry,
}

impl UninitVar {
    pub fn init(self) -> InitVar {
        InitVar {
            var: self.var,
            reg: self.reg,
        }
    }
}

struct InitVar {
    var: u16,
    reg: Registry,
}

impl From<&InitVar> for Var {
    fn from(value: &InitVar) -> Self {
        Var(value.var)
    }
}

impl From<&InitVar> for Expr {
    fn from(value: &InitVar) -> Self {
        Expr::Var(Var(value.var))
    }
}

impl Drop for InitVar {
    fn drop(&mut self) {
        self.reg.free(self.var);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    None,
    Untyped,
    Full,
}

struct Builder {
    reg: Registry,
    instrs: Vec<Instr>,

    checks: Check,
}

macro_rules! check_full {
    ($b:expr, $block:block) => {
        if $b.checks == Check::Full {
            $block
        }
    };

    ($b:expr, $stmt:stmt) => {
        if $b.checks == Check::Full {
            $stmt
        }
    };
}

macro_rules! check_untyped {
    ($b:expr, $block:block) => {
        if $b.checks == Check::Full || $b.checks == Check::Untyped {
            $block
        }
    };

    ($b:expr, $stmt:stmt) => {
        if $b.checks == Check::Full || $b.checks == Check::Untyped {
            $stmt
        }
    };
}

impl Builder {
    fn new(checks: Check) -> Self {
        Self {
            reg: Registry::default(),
            instrs: Vec::new(),
            checks,
        }
    }

    fn assert(&mut self, expr: impl Into<Expr>, msg: impl ToString) {
        let expr = expr.into();
        let msg = msg.to_string();

        self.instrs.push(Instr::Assert { expr, msg })
    }

    fn alloc_k(&mut self, size: u32) {
        self.instrs.push(Instr::AllocK { size });
    }

    fn alloc_d(&mut self, size: impl Into<Expr>) {
        self.instrs.push(Instr::AllocD { size: size.into() });
    }

    fn reserve_k(&mut self, size: u32) -> InitVar {
        let var = self.reg.var().init();

        self.instrs.push(Instr::ReserveK {
            into: Var::from(&var),
            size,
        });

        var
    }

    fn write_k(&mut self, func: impl Into<FuncK>, expr: impl Into<Expr>) {
        self.instrs.push(Instr::WriteK {
            func: func.into(),
            expr: expr.into(),
        });
    }

    fn write_d(&mut self, func: impl Into<FuncD>, expr: impl Into<Expr>, size: impl Into<Expr>) {
        self.instrs.push(Instr::WriteD {
            func: func.into(),
            expr: expr.into(),
            size: size.into(),
        });
    }

    fn write_reserved_k(
        &mut self,
        func: impl Into<FuncK>,
        at: impl Into<Expr>,
        expr: impl Into<Expr>,
    ) {
        self.instrs.push(Instr::WriteReservedK {
            func: func.into(),
            at: at.into(),
            expr: expr.into(),
        });
    }

    fn read_k(&mut self, func: impl Into<FuncK>) -> InitVar {
        let var = self.reg.var().init();

        self.instrs.push(Instr::ReadK {
            into: Var::from(&var),
            func: func.into(),
        });

        var
    }

    fn read_d(&mut self, func: impl Into<FuncD>, size: impl Into<Expr>) -> InitVar {
        let var = self.reg.var().init();

        self.instrs.push(Instr::ReadD {
            into: Var::from(&var),
            func: func.into(),
            size: size.into(),
        });

        var
    }

    fn expr(&mut self, expr: impl Into<Expr>) -> InitVar {
        let var = self.reg.var().init();

        self.instrs.push(Instr::Expr {
            into: Var::from(&var),
            expr: expr.into(),
        });

        var
    }

    fn assign(&mut self, var: &InitVar, expr: impl Into<Expr>) {
        self.instrs.push(Instr::Assign {
            into: Var::from(var),
            expr: expr.into(),
        });
    }

    fn assign_index(&mut self, var: &InitVar, index: impl Into<Expr>, expr: impl Into<Expr>) {
        self.instrs.push(Instr::AssignIndex {
            into: Var::from(var),
            index: index.into(),
            expr: expr.into(),
        });
    }

    fn iter_range(
        &mut self,
        start: impl Into<Expr>,
        end: impl Into<Expr>,
        block: impl FnOnce(&mut Builder, &InitVar),
    ) {
        let var = self.reg.var().init();
        let block = self.block(|b| block(b, &var));

        self.instrs.push(Instr::IterRange {
            into: Var::from(&var),
            start: start.into(),
            end: end.into(),
            block,
        });
    }

    fn iter_map(
        &mut self,
        map: impl Into<Expr>,
        block: impl FnOnce(&mut Builder, &InitVar, &InitVar),
    ) {
        let index = self.reg.var().init();
        let value = self.reg.var().init();
        let block = self.block(|b| block(b, &index, &value));

        self.instrs.push(Instr::IterMap {
            index: Var::from(&index),
            value: Var::from(&value),
            map: map.into(),
            block,
        });
    }

    fn block(&mut self, block: impl FnOnce(&mut Builder)) -> Block {
        let start = self.instrs.len();
        block(self);

        Block {
            instrs: self.instrs.split_off(start),
        }
    }

    fn check_type(&mut self, expr: impl Into<Expr>, ty: &'static str) {
        self.assert(expr.into().ty().eq(ty), format!("expected {ty}"));
    }

    fn check_range(&mut self, expr: impl Into<Expr>, range: &Range) {
        let expr = expr.into();

        if let Some(exact) = range.exact() {
            self.assert(expr.clone().eq(exact), format!("not equal to {exact}"));
        } else {
            match (range.min, range.max) {
                (Some(min), Some(max)) => self.assert(
                    expr.clone().ge(min).and(expr.clone().le(max)),
                    format!("value out of range [{min}, {max}]"),
                ),

                (Some(min), None) => {
                    self.assert(expr.clone().ge(min), format!("value less than {min}"))
                }

                (None, Some(max)) => {
                    self.assert(expr.clone().le(max), format!("value greater than {max}"))
                }

                (None, None) => {}
            }
        }
    }

    fn check_utf8(&mut self, expr: impl Into<Expr>) {
        self.assert(expr.into().utf8(), "not a valid utf8 string");
    }

    fn ser(b: &mut Builder, ty: Type, from: Expr) {
        match ty {
            Type::Number(ty) => {
                check_full!(b, b.check_type(from.clone(), "number"));
                check_untyped!(b, b.check_range(from.clone(), &ty.range));

                if !matches!(ty.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                    check_untyped!(b, b.assert(from.clone().eq(from.clone()), "value is nan"))
                }

                b.alloc_k(ty.kind.size());
                b.write_k(ty.kind, from);
            }

            Type::Vector(ty) => {
                check_full!(b, b.check_type(from.clone(), "vector"));

                Builder::ser(b, Type::Number(ty.x), from.clone().index("x"));
                Builder::ser(b, Type::Number(ty.y), from.clone().index("y"));

                if let Some(z) = ty.z {
                    Builder::ser(b, Type::Number(z), from.clone().index("z"));
                }
            }

            Type::BinaryString(ty) => {
                check_full!(b, b.check_type(from.clone(), "string"));

                if let Some(len) = ty.len.exact() {
                    check_untyped!(b, b.check_range(from.clone().len(), &ty.len));

                    b.alloc_k(len as u32);
                    b.write_d(FuncD::String, from, len);
                } else {
                    let len = b.expr(from.clone().len());
                    check_untyped!(b, b.check_range(&len, &ty.len));

                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);
                    b.alloc_d(&len);
                    b.write_d(FuncD::String, from, &len);
                }
            }

            Type::Utf8String(ty) => {
                check_full!(b, b.check_type(from.clone(), "string"));
                check_untyped!(b, b.check_utf8(from.clone()));

                if let Some(len) = ty.len.exact() {
                    check_untyped!(b, b.check_range(from.clone().len(), &ty.len));

                    b.alloc_k(len as u32);
                    b.write_d(FuncD::String, from, len);
                } else {
                    let len = b.expr(from.clone().len());
                    check_untyped!(b, b.check_range(&len, &ty.len));

                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);
                    b.alloc_d(&len);
                    b.write_d(FuncD::String, from, &len);
                }
            }

            Type::Array(ty) => {
                check_full!(b, b.check_type(from.clone(), "table"));

                if let Some(len) = ty.len.exact() {
                    check_untyped!(b, b.check_range(from.clone().len(), &ty.len));

                    b.iter_range(1f64, len, |b, index| {
                        let item = b.expr(from.clone().index(index));
                        Builder::ser(b, *ty.item, Expr::from(&item));
                    });
                } else {
                    let len = b.expr(from.clone().len());
                    check_untyped!(b, b.check_range(&len, &ty.len));

                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);

                    b.iter_range(1f64, &len, |b, index| {
                        let item = b.expr(from.clone().index(index));
                        Builder::ser(b, *ty.item, Expr::from(&item));
                    });
                }
            }

            Type::Set(ty) => {
                check_full!(b, b.check_type(from.clone(), "table"));

                let loc = b.reserve_k(ty.len.len_kind().size());
                let len = b.expr(0f64);

                b.iter_map(from, |b, value, _| {
                    b.assign(&len, Expr::from(&len).add(1f64));
                    Builder::ser(b, *ty.item, Expr::from(value));
                });

                check_untyped!(b, b.check_range(&len, &ty.len));
                b.write_reserved_k(ty.len.len_kind(), &loc, &len);
            }

            Type::Map(ty) => {
                check_full!(b, b.check_type(from.clone(), "table"));

                let loc = b.reserve_k(ty.len.len_kind().size());
                let len = b.expr(0f64);

                b.iter_map(from, |b, index, value| {
                    b.assign(&len, Expr::from(&len).add(1f64));
                    Builder::ser(b, *ty.index, Expr::from(index));
                    Builder::ser(b, *ty.value, Expr::from(value));
                });

                check_untyped!(b, b.check_range(&len, &ty.len));
                b.write_reserved_k(ty.len.len_kind(), &loc, &len);
            }

            Type::Struct(ty) => {
                check_full!(b, b.check_type(from.clone(), "table"));

                for (name, ty) in &ty.fields {
                    let field = b.expr(from.clone().index(name.as_str()));
                    Builder::ser(b, ty.clone(), Expr::from(&field));
                }
            }
        }
    }

    fn des(b: &mut Builder, ty: Type) -> InitVar {
        match ty {
            Type::Number(ty) => {
                let value = b.read_k(ty.kind);
                check_untyped!(b, b.check_range(&value, &ty.range));

                if !matches!(ty.kind, NumberKind::NaNF32 | NumberKind::NaNF64) {
                    check_untyped!(b, b.assert(Expr::from(&value).eq(&value), "value is nan"))
                }

                value
            }

            Type::Vector(ty) => {
                let x = Builder::des(b, Type::Number(ty.x));
                let y = Builder::des(b, Type::Number(ty.y));
                let z = ty.z.map(|z| Builder::des(b, Type::Number(z)));

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
                    check_full!(b, b.check_range(&len, &ty.len));

                    b.read_d(FuncD::String, &len)
                }
            }

            Type::Utf8String(ty) => {
                if let Some(len) = ty.len.exact() {
                    let str = b.read_d(FuncD::String, len);
                    check_full!(b, b.check_utf8(&str));

                    str
                } else {
                    let len = b.read_k(ty.len.len_kind());
                    check_full!(b, b.check_range(&len, &ty.len));

                    let str = b.read_d(FuncD::String, &len);
                    check_full!(b, b.check_utf8(&str));

                    str
                }
            }

            Type::Array(ty) => {
                if let Some(len) = ty.len.exact() {
                    let arr = b.expr(Expr::Array(Box::new(Expr::from(len))));
                    b.iter_range(1f64, len, |b, index| {
                        let value = Builder::des(b, *ty.item);
                        b.assign_index(&arr, index, &value);
                    });

                    arr
                } else {
                    let len = b.read_k(ty.len.len_kind());
                    check_full!(b, b.check_range(&len, &ty.len));

                    let arr = b.expr(Expr::Array(Box::new(Expr::from(&len))));
                    b.iter_range(1f64, &len, |b, index| {
                        let value = Builder::des(b, *ty.item);
                        b.assign_index(&arr, index, &value);
                    });

                    arr
                }
            }

            Type::Set(ty) => {
                let len = b.read_k(ty.len.len_kind());
                check_full!(b, b.check_range(&len, &ty.len));

                let set = b.expr(Expr::Table);
                b.iter_range(1f64, &len, |b, _| {
                    let value = Builder::des(b, *ty.item);
                    b.assign_index(&set, &value, Expr::Boolean(true));
                });

                set
            }

            Type::Map(ty) => {
                let len = b.read_k(ty.len.len_kind());
                check_full!(b, b.check_range(&len, &ty.len));

                let map = b.expr(Expr::Table);
                b.iter_range(1f64, &len, |b, _| {
                    let key = Builder::des(b, *ty.index);
                    let value = Builder::des(b, *ty.value);
                    b.assign_index(&map, &key, &value);
                });

                map
            }

            Type::Struct(ty) => {
                let mut fields = Vec::new();

                for (name, ty) in &ty.fields {
                    let field = Builder::des(b, ty.clone());
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
}
