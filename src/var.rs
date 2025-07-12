use std::{cell::RefCell, fmt::Display, rc::Rc};

#[derive(Debug, Clone, Copy)]
pub struct Var(pub u16);

impl Display for Var {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "v{}", self.0)
    }
}

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

impl From<&InitVar> for Var {
    fn from(value: &InitVar) -> Self {
        Var(value.var)
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
