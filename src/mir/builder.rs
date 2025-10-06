use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::mir::{Block, Expr, FuncD, FuncK, Instr, Var};

#[derive(Default)]
struct RegistryInner {
    next: u16,
    free: Vec<u16>,
}

#[derive(Default, Clone)]
struct Registry {
    inner: Rc<RefCell<RegistryInner>>,
}

impl Registry {
    fn var(&self) -> TVar {
        let mut inner = self.inner.borrow_mut();

        if let Some(var) = inner.free.pop() {
            TVar {
                reg: self.clone(),
                var: Var { id: var },
            }
        } else {
            let var = inner.next;
            inner.next += 1;

            TVar {
                reg: self.clone(),
                var: Var { id: var },
            }
        }
    }

    fn free(&self, var: Var) {
        self.inner.borrow_mut().free.push(var.id);
    }
}

#[must_use]
pub struct TVar {
    reg: Registry,
    var: Var,
}

impl TVar {
    pub fn expr(&self) -> Expr {
        Expr::Var(self.var)
    }
}

impl From<&TVar> for Expr {
    fn from(value: &TVar) -> Self {
        value.expr()
    }
}

impl From<&TVar> for Var {
    fn from(value: &TVar) -> Self {
        value.var
    }
}

impl Drop for TVar {
    fn drop(&mut self) {
        self.reg.free(self.var);
    }
}

#[derive(Default)]
pub struct Builder {
    reg: Registry,
    out: Vec<Instr>,
}

impl Builder {
    pub fn build(self) -> Block {
        Block { instrs: self.out }
    }

    pub fn block(&mut self, block: impl FnOnce(&mut Self)) -> Block {
        let start = self.out.len();
        block(self);

        Block {
            instrs: self.out.split_off(start),
        }
    }

    pub fn var(&mut self) -> TVar {
        self.reg.var()
    }

    pub fn instr(&mut self, instr: Instr) {
        self.out.push(instr);
    }

    pub fn export(&mut self, path: impl Display, name: impl Display, expr: impl Into<Expr>) {
        let path = format!("{path}.{name}");
        let expr = expr.into();

        self.instr(Instr::Export { path, expr });
    }

    pub fn init(&mut self, src: impl Into<Expr>) -> TVar {
        let var = self.var();
        let dst = Var::from(&var);
        let src = src.into();

        self.instr(Instr::Init { dst, src });

        var
    }

    pub fn assign(&mut self, dst: &TVar, src: impl Into<Expr>) {
        let dst = Var::from(dst);
        let src = src.into();

        self.instr(Instr::Assign { dst, src });
    }

    pub fn assign_index(&mut self, dst: &TVar, idx: impl Into<Expr>, src: impl Into<Expr>) {
        let dst = Var::from(dst);
        let idx = idx.into();
        let src = src.into();

        self.instr(Instr::AssignIndex { dst, idx, src });
    }

    pub fn call(&mut self, call: Expr) {
        debug_assert!(matches!(call, Expr::Call(..) | Expr::Namecall(..)));

        self.instr(Instr::Call { call });
    }

    pub fn ret(&mut self, exprs: Vec<impl Into<Expr>>) {
        let exprs = exprs.into_iter().map(Into::into).collect();

        self.instr(Instr::Return { exprs });
    }

    pub fn for_range(
        &mut self,
        start: impl Into<Expr>,
        end: impl Into<Expr>,
        block: impl FnOnce(&mut Self, &TVar),
    ) {
        let dst = self.var();
        let start = start.into();
        let end = end.into();
        let block = self.block(|b| block(b, &dst));

        self.instr(Instr::ForRange {
            dst: dst.var,
            start,
            end,
            block,
        });
    }

    pub fn for_table(
        &mut self,
        table: impl Into<Expr>,
        block: impl FnOnce(&mut Self, &TVar, &TVar),
    ) {
        let idx = self.var();
        let val = self.var();
        let table = table.into();
        let block = self.block(|b| block(b, &idx, &val));

        self.instr(Instr::ForTable {
            idx: idx.var,
            val: val.var,
            table,
            block,
        });
    }

    pub fn branch(
        &mut self,
        cond: impl Into<Expr>,
        then_block: impl FnOnce(&mut Self),
        else_block: impl FnOnce(&mut Self),
    ) {
        let cond = cond.into();
        let then_block = self.block(then_block);
        let else_block = self.block(else_block);

        self.instr(Instr::Branch {
            cond,
            then_block,
            else_block,
        });
    }

    pub fn function<const ARGS: usize>(
        &mut self,
        block: impl FnOnce(&mut Self, &[TVar; ARGS]),
    ) -> TVar {
        let dst = self.var();
        let vars: [_; ARGS] = std::array::from_fn(|_| self.var());
        let block = self.block(|b| block(b, &vars));
        let args = vars.map(|v| v.var).to_vec();

        self.instr(Instr::Function {
            dst: dst.var,
            args,
            block,
        });

        dst
    }

    pub fn function_n(&mut self, n: usize, block: impl FnOnce(&mut Self, &[TVar])) -> TVar {
        let dst = self.var();
        let vars: Vec<TVar> = (0..n).map(|_| self.var()).collect();
        let block = self.block(|b| block(b, &vars));
        let args = vars.iter().map(|v| v.var).collect();

        self.instr(Instr::Function {
            dst: dst.var,
            args,
            block,
        });

        dst
    }
}

pub struct Ctx {
    pub buf: TVar,
    pub pos: TVar,
    pub len: TVar,
}

impl Builder {
    pub fn check(&mut self, ctx: &Ctx, size: impl Into<Expr>) {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let len = ctx.len.var;
        let size = size.into();

        self.instr(Instr::Check {
            buf,
            pos,
            len,
            size,
        });
    }

    pub fn reserve(&mut self, ctx: &Ctx, size: impl Into<Expr>) -> TVar {
        let dst = self.var();
        let pos = ctx.pos.var;
        let size = size.into();

        self.instr(Instr::Reserve {
            dst: dst.var,
            pos,
            size,
        });

        dst
    }

    pub fn write_k(&mut self, ctx: &Ctx, func: FuncK, src: impl Into<Expr>) {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let src = src.into();

        self.instr(Instr::WriteK {
            func,
            buf,
            pos,
            src,
        });
    }

    pub fn write_d(&mut self, ctx: &Ctx, func: FuncD, src: impl Into<Expr>, size: impl Into<Expr>) {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let src = src.into();
        let size = size.into();

        self.instr(Instr::WriteD {
            func,
            buf,
            pos,
            src,
            size,
        });
    }

    pub fn write_reserved_k(&mut self, ctx: &Ctx, func: FuncK, src: impl Into<Expr>) {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let src = src.into();

        self.instr(Instr::WriteReservedK {
            func,
            buf,
            pos,
            src,
        });
    }

    pub fn read_k(&mut self, ctx: &Ctx, func: FuncK) -> TVar {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let var = self.var();
        let dst = var.var;

        self.instr(Instr::ReadK {
            func,
            buf,
            pos,
            dst,
        });

        var
    }

    pub fn read_d(&mut self, ctx: &Ctx, func: FuncD, size: impl Into<Expr>) -> TVar {
        let buf = ctx.buf.var;
        let pos = ctx.pos.var;
        let size = size.into();
        let var = self.var();
        let dst = var.var;

        self.instr(Instr::ReadD {
            func,
            buf,
            pos,
            size,
            dst,
        });

        var
    }
}

impl Builder {
    pub fn assert(&mut self, cond: impl Into<Expr>, msg: impl Into<String>) {
        let error = Expr::Global("error").call(vec![Expr::from(msg.into())]);
        self.branch(cond.into().not(), |b| b.call(error), |_| {})
    }
}
