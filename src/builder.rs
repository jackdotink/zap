use std::{cell::RefCell, fmt::Display, rc::Rc};

use crate::ir::{Block, Expr, FuncD, FuncK, Instr, Var};

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

impl Display for Builder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for instr in &self.out {
            write!(f, "{instr}")?;
        }

        Ok(())
    }
}

impl Builder {
    pub fn block(&mut self, block: impl FnOnce(&mut Builder)) -> Block {
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

        self.out.push(Instr::Assert { expr, msg })
    }

    pub fn alloc_k(&mut self, size: u32) {
        self.out.push(Instr::AllocK { size });
    }

    pub fn alloc_d(&mut self, size: impl Into<Expr>) {
        self.out.push(Instr::AllocD { size: size.into() });
    }

    pub fn reserve_k(&mut self, size: u32) -> InitVar {
        let var = self.reg.var().init();

        self.out.push(Instr::ReserveK {
            into: Var::from(&var),
            size,
        });

        var
    }

    pub fn write_k(&mut self, func: impl Into<FuncK>, expr: impl Into<Expr>) {
        self.out.push(Instr::WriteK {
            func: func.into(),
            expr: expr.into(),
        });
    }

    pub fn write_d(
        &mut self,
        func: impl Into<FuncD>,
        expr: impl Into<Expr>,
        size: impl Into<Expr>,
    ) {
        self.out.push(Instr::WriteD {
            func: func.into(),
            expr: expr.into(),
            size: size.into(),
        });
    }

    pub fn write_reserved_k(
        &mut self,
        func: impl Into<FuncK>,
        at: impl Into<Expr>,
        expr: impl Into<Expr>,
    ) {
        self.out.push(Instr::WriteReservedK {
            func: func.into(),
            at: at.into(),
            expr: expr.into(),
        });
    }

    pub fn read_k(&mut self, func: impl Into<FuncK>) -> InitVar {
        let var = self.reg.var().init();

        self.out.push(Instr::ReadK {
            into: Var::from(&var),
            func: func.into(),
        });

        var
    }

    pub fn read_d(&mut self, func: impl Into<FuncD>, size: impl Into<Expr>) -> InitVar {
        let var = self.reg.var().init();

        self.out.push(Instr::ReadD {
            into: Var::from(&var),
            func: func.into(),
            size: size.into(),
        });

        var
    }

    pub fn expr(&mut self, expr: impl Into<Expr>) -> InitVar {
        let var = self.reg.var().init();

        self.out.push(Instr::Expr {
            into: Var::from(&var),
            expr: expr.into(),
        });

        var
    }

    pub fn assign(&mut self, var: &InitVar, expr: impl Into<Expr>) {
        self.out.push(Instr::Assign {
            into: Var::from(var),
            expr: expr.into(),
        });
    }

    pub fn assign_index(&mut self, var: &InitVar, index: impl Into<Expr>, expr: impl Into<Expr>) {
        self.out.push(Instr::AssignIndex {
            into: Var::from(var),
            index: index.into(),
            expr: expr.into(),
        });
    }

    pub fn iter_range(
        &mut self,
        start: impl Into<Expr>,
        end: impl Into<Expr>,
        block: impl FnOnce(&mut Builder, &InitVar),
    ) {
        let var = self.reg.var().init();
        let block = self.block(|b| block(b, &var));

        self.out.push(Instr::IterRange {
            into: Var::from(&var),
            start: start.into(),
            end: end.into(),
            block,
        });
    }

    pub fn iter_map(
        &mut self,
        map: impl Into<Expr>,
        block: impl FnOnce(&mut Builder, &InitVar, &InitVar),
    ) {
        let index = self.reg.var().init();
        let value = self.reg.var().init();
        let block = self.block(|b| block(b, &index, &value));

        self.out.push(Instr::IterMap {
            index: Var::from(&index),
            value: Var::from(&value),
            map: map.into(),
            block,
        });
    }

    pub fn branch(
        &mut self,
        cond: impl Into<Expr>,
        then_block: impl FnOnce(&mut Builder),
        else_block: impl FnOnce(&mut Builder),
    ) {
        let then_block = self.block(then_block);
        let else_block = self.block(else_block);

        self.out.push(Instr::Branch {
            cond: cond.into(),
            then_block,
            else_block,
        });
    }

    pub fn local_function<const ARGS: usize>(
        &mut self,
        block: impl FnOnce(&mut Builder, &[InitVar; ARGS]),
    ) -> InitVar {
        let into = self.reg.var().init();
        let args: [_; ARGS] = std::array::from_fn(|_| self.reg.var().init());
        let block = self.block(|b| block(b, &args));

        self.out.push(Instr::LocalFunction {
            into: Var::from(&into),
            args: args.iter().map(Var::from).collect(),
            body: block,
        });

        into
    }

    pub fn export_table(&mut self, path: impl ToString) {
        self.out.push(Instr::ExportTable {
            path: path.to_string(),
        });
    }

    pub fn export_function(
        &mut self,
        path: impl ToString,
        args: Vec<String>,
        rets: Vec<String>,
        block: impl FnOnce(&mut Builder, &[InitVar]),
    ) {
        let path = path.to_string();
        let vars = args.iter().map(|_| self.var().init()).collect::<Vec<_>>();
        let body = self.block(|b| block(b, &vars));
        let args = vars.iter().map(Var::from).zip(args).collect::<Vec<_>>();

        self.out.push(Instr::ExportFunction {
            path,
            args,
            rets,
            body,
        });
    }
}
