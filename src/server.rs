use crate::{
    builder::{Builder, InitVar},
    ir::Expr,
    serdes,
    types::{Event, EventFrom, Item, Type},
};

pub fn server(s: Server, items: &[(String, Item)]) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;

    let mut b = Builder::default();

    for (name, item) in items {
        self::item(s.push_location(name), &mut b, item)
    }

    let mut s = String::new();
    writeln!(s, "{}", include_str!("header.luau"))?;
    writeln!(s, "local result = {{}}")?;
    writeln!(s, "local folder = Instance.new('Folder')")?;
    writeln!(s, "folder.Name = 'Z'")?;
    writeln!(s, "folder.Parent = game.ReplicatedStorage")?;
    writeln!(s, "{b}")?;
    writeln!(s, "return result")?;

    Ok(s)
}

#[derive(Debug, Clone)]
pub struct Server {
    pub apicheck: serdes::ApiCheck,
    pub location: String,
}

impl Server {
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

fn ser(s: &Server, b: &mut Builder, ty: Type, from: Expr) {
    let ser = serdes::Ser {
        apicheck: s.apicheck,
    };

    serdes::ser(ser, b, ty, from)
}

fn des(s: &Server, b: &mut Builder, ty: Type) -> InitVar {
    let des = serdes::Des { check: false };

    serdes::des(des, b, ty)
}

pub fn item(s: Server, b: &mut Builder, item: &Item) {
    match item {
        Item::Table(table) => self::table(s, b, table),
        Item::Event(event) => self::event(s, b, event),
    }
}

pub fn table(s: Server, b: &mut Builder, items: &[(String, Item)]) {
    b.export_table(s.location.clone());

    for (name, item) in items {
        self::item(s.push_location(name), b, item)
    }
}

pub fn event(s: Server, b: &mut Builder, event: &Event) {
    b.export_table(s.location.clone());

    match event.from {
        EventFrom::Server => event_send(s, b, event),
        EventFrom::Client => event_recv(s, b, event),
    }
}

fn event_remote(b: &mut Builder, event: &Event) -> InitVar {
    let var = b.var().init();
    let name = event.name;

    b.stmt(format!("local {var} = Instance.new('RemoteEvent', folder)"));
    b.stmt(format!("{var}.Name = '{name}'"));

    var
}

fn event_recv(s: Server, b: &mut Builder, event: &Event) {
    match event.data.len() {
        0 => event_recv_iter_0data(&s, b, event),
        _ => event_recv_iter_ndata(&s, b, event),
    }
}

fn event_recv_queue_0data(s: &Server, b: &mut Builder, event: &Event) -> InitVar {
    let remote = event_remote(b, event);
    let queue = b.expr(Expr::Table);

    let on_event = b.local_function(|b, [player]| {
        b.stmt(format!("table.insert({queue}, {player})"));
    });

    b.stmt(format!("{remote}.OnServerEvent:Connect({on_event})"));
    queue
}

fn event_recv_queue_ndata(s: &Server, b: &mut Builder, event: &Event) -> InitVar {
    let remote = event_remote(b, event);
    let queue = b.expr(Expr::Table);

    let on_event = b.local_function(|b, [player, buf]| {
        b.stmt(format!("local buf, pos = {buf}, 0"));
        b.stmt(format!("table.insert({queue}, {player})"));

        for ty in &event.data {
            let value = des(s, b, ty.clone());
            b.stmt(format!("table.insert({queue}, {value})"));
        }
    });

    b.stmt(format!(
        "{remote}.OnServerEvent:Connect(function(player, buf) pcall({on_event}, player, buf) end)"
    ));

    queue
}

fn event_recv_iter_0data(s: &Server, b: &mut Builder, event: &Event) {
    let queue = event_recv_queue_0data(s, b, event);

    b.export_function(
        s.function("iter"),
        vec![],
        vec!["{ Player }".to_string()],
        |b, _| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table);
            b.stmt(format!("return {captured}"));
        },
    )
}

fn event_recv_iter_ndata(s: &Server, b: &mut Builder, event: &Event) {
    let n = event.data.len();
    let queue = event_recv_queue_ndata(s, b, event);
    let tys = event
        .data
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    b.export_function(
        s.function("iter"),
        vec![],
        vec![
            format!("({{ any }}, number) -> (number, Player, {tys})"),
            "{ any }".to_string(),
            "number".to_string(),
        ],
        |b, _| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table);

            let iter = b.local_function(|b, [captured, i]| {
                let cond = format!("{captured}[{i}]");
                let rets = (0..=n)
                    .map(|j| format!(", {captured}[{i} + {j}]"))
                    .collect::<String>();
                let body = format!("return {i} + {}{rets}", n + 1);

                b.stmt(format!("if {cond} then {body} end"));
            });

            b.stmt(format!("return {iter}, {captured}, 1"));
        },
    );
}

fn event_send(s: Server, b: &mut Builder, event: &Event) {
    match event.data.len() {
        0 => event_send_0data(&s, b, event),
        _ => event_send_ndata(&s, b, event),
    }
}

fn event_send_0data(s: &Server, b: &mut Builder, event: &Event) {
    let remote = event_remote(b, event);

    b.export_function(
        s.function("fire"),
        vec!["Player".to_string()],
        vec![],
        |b, args| {
            let player = &args[0];
            b.stmt(format!("{remote}:FireClient({player})"));
        },
    );
}

fn event_send_ndata(s: &Server, b: &mut Builder, event: &Event) {
    let remote = event_remote(b, event);
    let args = event
        .data
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    b.export_function(
        s.function("fire"),
        std::iter::once("Player".to_string())
            .chain(args.clone())
            .collect(),
        vec![],
        |b, args| {
            let player = &args[0];
            b.stmt("pos = 0");

            for (i, ty) in event.data.iter().enumerate() {
                ser(s, b, ty.clone(), args[i + 1].expr());
            }

            b.stmt("local out = buffer.create(pos)");
            b.stmt("buffer.copy(out, 0, buf, 0, pos)");
            b.stmt(format!("{remote}:FireClient({player}, out)"));
        },
    )
}
