use std::rc::Rc;

use crate::{
    hir::{Event, Item, Table},
    mir::{
        Expr,
        builder::{Builder, InitVar},
        serdes::{Des, Ser},
    },
    shared::{ApiCheck, NetworkSide, Options},
};

mod iter;
mod send;

#[derive(Clone)]
pub struct Server {
    pub location: String,
    pub options: Rc<Options>,

    pub ser: Ser,
    pub des: Des,
}

impl Default for Server {
    fn default() -> Self {
        let location = "result".to_string();
        let options = Rc::new(Options::default());

        let ser = Ser {
            options: Rc::new(Options::default()),
            native: true,
        };

        let des = Des {
            options: Rc::new(Options::default()),
            native: true,
            check: true,
        };

        Server {
            location,
            options,
            ser,
            des,
        }
    }
}

impl Server {
    fn item(&self, b: &mut Builder, item: &Item) {
        match item {
            Item::Table(table) => self.table(b, table),
            Item::Event(event) => self.event(b, event),
        }
    }

    pub fn table(&self, b: &mut Builder, table: &Table) {
        for (name, item) in table.items.iter() {
            self.export(b, name, Expr::Table(vec![]));
            self.child(&table.options, name).item(b, item);
        }
    }

    fn event(&self, b: &mut Builder, event: &Event) {
        match event.from {
            NetworkSide::Server => self.event_send(b, event),
            NetworkSide::Client => self.event_recv_iter(b, event),
        }
    }

    fn child(&self, options: &Rc<Options>, name: &str) -> Self {
        let options = Rc::clone(options);

        let mut ser = self.ser.clone();
        ser.options = Rc::clone(&options);

        let mut des = self.des.clone();
        des.options = Rc::clone(&options);

        Server {
            location: self.location.clone() + "." + name,
            options,
            ser,
            des,
        }
    }

    fn name(&self, words: &'static str) -> String {
        self.options.casing().fmt(words)
    }

    fn export(&self, b: &mut Builder, name: &str, expr: impl Into<Expr>) {
        let expr = expr.into();
        let location = &self.location;
        b.stmt(format!("{location}.{name} = {expr}"));
    }

    fn remote(&self, b: &mut Builder, event: &Event) -> InitVar {
        let var = b.var().init();
        let uuid = event.uuid;

        b.stmt(format!("local {var} = Instance.new('RemoteEvent')"));
        b.stmt(format!("{var}.Name = '{uuid}'"));
        b.stmt(format!("{var}.Parent = folder"));

        var
    }
}
