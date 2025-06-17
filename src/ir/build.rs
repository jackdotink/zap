use std::{cell::RefCell, rc::Rc};

use crate::{
    ir::{Block, Expr, FuncD, FuncK, Instr, Var},
    types::Type,
};

pub fn build(ty: Type) -> Block {
    let mut builder = Builder::default();
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

#[derive(Default)]
struct Builder {
    reg: Registry,
    instrs: Vec<Instr>,
}

impl Builder {
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

    fn ser(b: &mut Builder, ty: Type, from: Expr) {
        match ty {
            Type::Number(ty) => {
                b.alloc_k(ty.kind.size());
                b.write_k(ty.kind, from);
            }

            Type::Vector(ty) => {
                Builder::ser(b, Type::Number(ty.x), from.clone().index("x"));
                Builder::ser(b, Type::Number(ty.y), from.clone().index("y"));

                if let Some(z) = ty.z {
                    Builder::ser(b, Type::Number(z), from.clone().index("z"));
                }
            }

            Type::BinaryString(ty) => {
                if let Some(len) = ty.len.exact() {
                    b.alloc_k(len as u32);
                    b.write_d(FuncD::String, from, len);
                } else {
                    let len = b.expr(from.clone().len());
                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);
                    b.alloc_d(&len);
                    b.write_d(FuncD::String, from, &len);
                }
            }

            Type::Utf8String(ty) => {
                if let Some(len) = ty.len.exact() {
                    b.alloc_k(len as u32);
                    b.write_d(FuncD::String, from, len);
                } else {
                    let len = b.expr(from.clone().len());
                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);
                    b.alloc_d(&len);
                    b.write_d(FuncD::String, from, &len);
                }
            }

            Type::Array(ty) => {
                if let Some(len) = ty.len.exact() {
                    b.iter_range(1f64, len, |b, index| {
                        let item = b.expr(from.clone().index(index));
                        Builder::ser(b, *ty.item, Expr::from(&item));
                    });
                } else {
                    let len = b.expr(from.clone().len());
                    b.alloc_k(ty.len.len_kind().size());
                    b.write_k(ty.len.len_kind(), &len);

                    b.iter_range(1f64, &len, |b, index| {
                        let item = b.expr(from.clone().index(index));
                        Builder::ser(b, *ty.item, Expr::from(&item));
                    });
                }
            }

            Type::Set(ty) => {
                if let Some(_len) = ty.len.exact() {
                    b.iter_map(from, |b, value, _| {
                        Builder::ser(b, *ty.item, Expr::from(value));
                    });
                } else {
                    let loc = b.reserve_k(ty.len.len_kind().size());
                    let len = b.expr(0f64);

                    b.iter_map(from, |b, value, _| {
                        b.assign(&len, Expr::from(&len) + 1f64.into());
                        Builder::ser(b, *ty.item, Expr::from(value));
                    });

                    b.write_reserved_k(ty.len.len_kind(), &loc, &len);
                }
            }

            Type::Map(ty) => {
                if let Some(_len) = ty.len.exact() {
                    b.iter_map(from, |b, index, value| {
                        Builder::ser(b, *ty.index, Expr::from(index));
                        Builder::ser(b, *ty.value, Expr::from(value));
                    })
                } else {
                    let loc = b.reserve_k(ty.len.len_kind().size());
                    let len = b.expr(0f64);

                    b.iter_map(from, |b, index, value| {
                        b.assign(&len, Expr::from(&len) + 1f64.into());
                        Builder::ser(b, *ty.index, Expr::from(index));
                        Builder::ser(b, *ty.value, Expr::from(value));
                    });

                    b.write_reserved_k(ty.len.len_kind(), &loc, &len);
                }
            }

            Type::Struct(ty) => {
                for (name, ty) in &ty.fields {
                    let field = b.expr(from.clone().index(name.as_str()));
                    Builder::ser(b, ty.clone(), Expr::from(&field));
                }
            }
        }
    }
}
