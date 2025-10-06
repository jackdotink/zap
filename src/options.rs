use std::{cell::RefCell, rc::Rc};

use crate::shared::{ApiCheck, Casing};

#[derive(Default, Clone)]
pub struct Options {
    pub apicheck: ApiCheck,
    pub casing: Casing,
}

#[derive(Default, Clone)]
pub struct Partial {
    pub apicheck: Option<ApiCheck>,
    pub casing: Option<Casing>,
}

struct NodeInner {
    pub partial: Partial,
    pub resolved: Option<Options>,
    pub parent: Option<Node>,
}

#[derive(Clone)]
pub struct Node(Rc<RefCell<NodeInner>>);

impl From<Partial> for Node {
    fn from(value: Partial) -> Self {
        Self(Rc::new(RefCell::new(NodeInner {
            partial: value,
            resolved: None,
            parent: None,
        })))
    }
}

impl Node {
    fn inner(&self) -> std::cell::RefMut<'_, NodeInner> {
        self.0.borrow_mut()
    }

    pub fn resolved(&self) -> Options {
        let mut inner = self.inner();
        if inner.resolved.is_none() {
            let parent = inner
                .parent
                .as_ref()
                .map_or(Default::default(), |p| p.resolved());

            inner.resolved = Some(Options {
                apicheck: inner.partial.apicheck.unwrap_or(parent.apicheck),
                casing: inner.partial.casing.unwrap_or(parent.casing),
            });
        }

        inner.resolved.as_ref().unwrap().clone()
    }

    pub fn set_parent(&self, parent: Node) {
        self.inner().parent = Some(parent)
    }
}
