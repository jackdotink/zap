use crate::{
    builder::{Builder, InitVar},
    ir::Expr,
    serdes,
    types::{Event, EventFrom, Item, Type},
};

pub fn client(c: Client, items: &[(String, Item)]) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;

    let mut b = Builder::default();

    for (name, item) in items {
        self::item(c.push_location(name), &mut b, item)
    }

    let mut s = String::new();
    writeln!(s, "{}", include_str!("header.luau"))?;
    writeln!(s, "local result = {{}}")?;
    writeln!(s, "local folder = game.ReplicatedStorage:WaitForChild('Z')")?;
    writeln!(s, "{b}")?;
    writeln!(s, "return result")?;

    Ok(s)
}

#[derive(Default, Debug, Clone)]
pub struct Client {
    pub apicheck: serdes::ApiCheck,
    pub location: String,
}

impl Client {
    pub fn push_location(&self, location: &str) -> Self {
        Self {
            apicheck: self.apicheck,
            location: self.location.clone() + "." + location,
        }
    }

    pub fn function(&self, name: &str) -> String {
        self.location.clone() + "." + name
    }
}

fn ser(c: &Client, b: &mut Builder, ty: Type, from: Expr) {
    let ser = serdes::Ser {
        apicheck: c.apicheck,
    };

    serdes::ser(ser, b, ty, from)
}

fn des(c: &Client, b: &mut Builder, ty: Type) -> InitVar {
    let des = serdes::Des { check: false };

    serdes::des(des, b, ty)
}

pub fn item(c: Client, b: &mut Builder, item: &Item) {
    match item {
        Item::Table(table) => self::table(c, b, table),
        Item::Event(event) => self::event(c, b, event),
    }
}

pub fn table(c: Client, b: &mut Builder, items: &[(String, Item)]) {
    b.export_table(c.location.clone());

    for (name, item) in items {
        self::item(c.push_location(name), b, item)
    }
}

fn event(c: Client, b: &mut Builder, event: &Event) {
    b.export_table(c.location.clone());

    match event.from {
        EventFrom::Server => event_recv(c, b, event),
        EventFrom::Client => event_send(c, b, event),
    }
}

fn event_remote(b: &mut Builder, event: &Event) -> InitVar {
    let var = b.var().init();
    let name = event.name;

    b.stmt(format!("local {var} = folder:WaitForChild(\"{name}\")"));

    var
}

fn event_recv(c: Client, b: &mut Builder, event: &Event) {
    match event.data.len() {
        0 => event_recv_iter_0data(&c, b, event),
        1 => event_recv_iter_1data(&c, b, event),
        _ => event_recv_iter_ndata(&c, b, event),
    }
}

fn event_recv_counter(c: &Client, b: &mut Builder, event: &Event) -> InitVar {
    let remote = event_remote(b, event);
    let counter = b.expr(Expr::Number(0f64));

    let on_event = b.local_function(|b, []| {
        b.assign(&counter, counter.expr().add(1f64));
    });

    b.stmt(format!("{remote}.OnClientEvent:Connect({on_event})"));
    counter
}

fn event_recv_queue(c: &Client, b: &mut Builder, event: &Event) -> InitVar {
    let remote = event_remote(b, event);
    let queue = b.expr(Expr::Table);

    let on_event = b.local_function(|b, [buf]| {
        b.stmt(format!("local buf, pos = {buf}, 0"));

        for ty in &event.data {
            let value = des(c, b, ty.clone());
            b.stmt(format!("table.insert({queue}, {value})"));
        }
    });

    b.stmt(format!("{remote}.OnClientEvent:Connect({on_event})"));
    queue
}

fn event_recv_iter_0data(c: &Client, b: &mut Builder, event: &Event) {
    let counter = event_recv_counter(c, b, event);

    b.export_function(
        c.function("iter"),
        vec![],
        vec![
            "(number, number) -> number".to_string(),
            "number".to_string(),
        ],
        |b, _| {
            let captured = b.expr(&counter);
            b.assign(&counter, Expr::Number(0f64));

            let iter = b.local_function(|b, [i]| {
                b.stmt(format!("if {i} < {captured} then return {i} + 1 end"));
            });

            b.stmt(format!("return {iter}, {captured}, 0"));
        },
    );
}

fn event_recv_iter_1data(c: &Client, b: &mut Builder, event: &Event) {
    let queue = event_recv_queue(c, b, event);

    b.export_function(
        c.function("iter"),
        vec![],
        vec![format!("{{ {} }}", event.data[0])],
        |b, _| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table);
            b.stmt(format!("return {captured}"));
        },
    );
}

fn event_recv_iter_ndata(c: &Client, b: &mut Builder, event: &Event) {
    let n = event.data.len();
    let queue = event_recv_queue(c, b, event);
    let tys = event
        .data
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    b.export_function(
        c.function("iter"),
        vec![],
        vec![
            format!("({{ any }}, number) -> (number, {tys})"),
            "{ any }".to_string(),
            "number".to_string(),
        ],
        |b, _| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table);

            let iter = b.local_function(|b, [captured, i]| {
                let cond = format!("{captured}[{i}]");
                let rets = (0..n)
                    .map(|j| format!(", {captured}[{i} + {j}]"))
                    .collect::<String>();
                let body = format!("return {i} + {n}{rets}");

                b.stmt(format!("if {cond} then {body} end"));
            });

            b.stmt(format!("return {iter}, {captured}, 1"));
        },
    )
}

fn event_send(c: Client, b: &mut Builder, event: &Event) {
    match event.data.len() {
        0 => event_send_0data(&c, b, event),
        _ => event_send_ndata(&c, b, event),
    }
}

fn event_send_0data(c: &Client, b: &mut Builder, event: &Event) {
    let remote = event_remote(b, event);

    b.export_function(c.function("fire"), vec![], vec![], |b, _| {
        b.stmt(format!("{remote}:FireServer()"));
    });
}

fn event_send_ndata(c: &Client, b: &mut Builder, event: &Event) {
    let remote = event_remote(b, event);
    let args = event
        .data
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    b.export_function(c.function("fire"), args, vec![], |b, args| {
        b.stmt("pos = 0");

        for (i, arg) in args.iter().enumerate() {
            let ty = event.data[i].clone();
            ser(c, b, ty, arg.expr());
        }

        b.stmt("local out = buffer.create(pos)");
        b.stmt("buffer.copy(out, 0, buf, 0, pos)");
        b.stmt(format!("{remote}:FireServer(out)"));
    });
}
