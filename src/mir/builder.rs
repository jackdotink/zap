use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::mir::{Block, Expr, FuncD, FuncK, Instr, Var};

#[derive(Default)]
struct RegistryInner {
    next: u16,
    free: Vec<u16>,
}

#[derive(Default, Clone)]
pub struct Registry {
    inner: Rc<RefCell<RegistryInner>>,
}

impl Registry {
    pub fn var(&self) -> UninitVar {
        let mut inner = self.inner.borrow_mut();

        if let Some(var) = inner.free.pop() {
            UninitVar {
                var,
                reg: self.clone(),
            }
        } else {
            let var = inner.next;
            inner.next += 1;
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
pub struct UninitVar {
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

pub struct InitVar {
    var: u16,
    reg: Registry,
}

impl InitVar {
    pub fn expr(&self) -> Expr {
        Expr::Var(Var(self.var))
    }
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

impl Display for InitVar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Var(self.var))
    }
}

impl Drop for InitVar {
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
    pub fn block(&mut self, block: impl FnOnce(&mut Self)) -> Block {
        let start = self.out.len();
        block(self);

        Block {
            instrs: self.out.split_off(start),
        }
    }

    pub fn var(&mut self) -> UninitVar {
        self.reg.var()
    }

    pub fn stmt(&mut self, stmt: impl ToString) {
        let stmt = stmt.to_string();

        self.out.push(Instr::Stmt { stmt });
    }

    pub fn assert(&mut self, expr: impl Into<Expr>, msg: impl ToString) {
        let expr = expr.into();
        let msg = msg.to_string();

        self.out.push(Instr::Assert { expr, msg });
    }

    pub fn alloc_k(&mut self, size: u32) {
        self.out.push(Instr::AllocK { size });
    }

    pub fn alloc_d(&mut self, size: impl Into<Expr>) {
        let size = size.into();

        self.out.push(Instr::AllocD { size });
    }

    pub fn reserve_k(&mut self, size: u32) -> InitVar {
        let var = self.var().init();
        let into = Var::from(&var);

        self.out.push(Instr::ReserveK { into, size });
        var
    }

    pub fn write_k(&mut self, func: impl Into<FuncK>, expr: impl Into<Expr>) {
        let func = func.into();
        let expr = expr.into();

        self.out.push(Instr::WriteK { func, expr });
    }

    pub fn write_d(
        &mut self,
        func: impl Into<FuncD>,
        expr: impl Into<Expr>,
        size: impl Into<Expr>,
    ) {
        let func = func.into();
        let expr = expr.into();
        let size = size.into();

        self.out.push(Instr::WriteD { func, expr, size });
    }

    pub fn write_reserved_k(
        &mut self,
        func: impl Into<FuncK>,
        at: impl Into<Expr>,
        expr: impl Into<Expr>,
    ) {
        let func = func.into();
        let at = at.into();
        let expr = expr.into();

        self.out.push(Instr::WriteReservedK { func, at, expr });
    }

    pub fn read_k(&mut self, func: impl Into<FuncK>) -> InitVar {
        let var = self.var().init();
        let into = Var::from(&var);
        let func = func.into();

        self.out.push(Instr::ReadK { into, func });
        var
    }

    pub fn read_d(&mut self, func: impl Into<FuncD>, size: impl Into<Expr>) -> InitVar {
        let var = self.var().init();
        let into = Var::from(&var);
        let func = func.into();
        let size = size.into();

        self.out.push(Instr::ReadD { into, func, size });
        var
    }

    pub fn expr(&mut self, expr: impl Into<Expr>) -> InitVar {
        let var = self.var().init();
        let into = Var::from(&var);
        let expr = expr.into();

        self.out.push(Instr::Expr { into, expr });
        var
    }

    pub fn assign(&mut self, into: &InitVar, expr: impl Into<Expr>) {
        let expr = expr.into();
        let into = Var::from(into);

        self.out.push(Instr::Assign { into, expr })
    }

    pub fn assign_index(&mut self, into: &InitVar, index: impl Into<Expr>, expr: impl Into<Expr>) {
        let into = Var::from(into);
        let index = index.into();
        let expr = expr.into();

        self.out.push(Instr::AssignIndex { into, index, expr });
    }

    pub fn for_range(
        &mut self,
        start: impl Into<Expr>,
        end: impl Into<Expr>,
        block: impl FnOnce(&mut Self, &InitVar),
    ) {
        let var = self.var().init();
        let block = self.block(|b| block(b, &var));

        let into = Var::from(&var);
        let start = start.into();
        let end = end.into();

        self.out.push(Instr::ForRange {
            into,
            start,
            end,
            block,
        });
    }

    pub fn for_table(
        &mut self,
        table: impl Into<Expr>,
        block: impl FnOnce(&mut Self, &InitVar, &InitVar),
    ) {
        let index = self.var().init();
        let value = self.var().init();
        let block = self.block(|b| block(b, &index, &value));

        let table = table.into();

        self.out.push(Instr::ForTable {
            index: Var::from(&index),
            value: Var::from(&value),
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

        self.out.push(Instr::Branch {
            cond,
            then_block,
            else_block,
        });
    }

    pub fn local_function<const ARGS: usize>(
        &mut self,
        block: impl FnOnce(&mut Self, &[InitVar; ARGS]),
    ) -> InitVar {
        let into = self.var().init();
        let vars: [_; ARGS] = std::array::from_fn(|_| self.var().init());
        let body = self.block(|b| block(b, &vars));
        let args = vars.iter().map(Var::from).collect();

        self.out.push(Instr::LocalFunction {
            into: Var::from(&into),
            args,
            body,
        });

        into
    }
}
