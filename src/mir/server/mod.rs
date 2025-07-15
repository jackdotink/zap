use crate::{
    hir::{Event, Item},
    mir::{
        Expr,
        builder::{Builder, InitVar},
        serdes::{Des, Ser},
    },
    shared::{ApiCheck, NetworkSide},
};

mod iter;
mod send;

#[derive(Clone)]
pub struct Server {
    pub location: String,

    pub ser: Ser,
    pub des: Des,
}

impl Server {
    pub fn new(apicheck: ApiCheck) -> Self {
        let location = "result".to_string();
        let ser = Ser {
            apicheck,
            native: false,
        };
        let des = Des {
            apicheck,
            native: false,
            check: false,
        };

        Server { location, ser, des }
    }

    pub fn item(&self, b: &mut Builder, item: &Item) {
        match item {
            Item::Table(table) => self.table(b, table),
            Item::Event(event) => self.event(b, event),
        }
    }

    fn table(&self, b: &mut Builder, table: &[(String, Item)]) {
        for (name, item) in table {
            self.export(b, name, Expr::Table(vec![]));
            self.location(name).item(b, item);
        }
    }

    fn event(&self, b: &mut Builder, event: &Event) {
        match event.from {
            NetworkSide::Server => self.event_send(b, event),
            NetworkSide::Client => self.event_recv_iter(b, event),
        }
    }

    fn location(&self, name: &str) -> Self {
        Server {
            location: self.location.clone() + "." + name,
            ser: self.ser.clone(),
            des: self.des.clone(),
        }
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
