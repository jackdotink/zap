use crate::{
    hir::{Buckets, Hir, Item, Size, SizeOf, Table},
    mir::{
        Expr,
        builder::{Builder, Ctx, TVar},
        serdes::{Des, Ser},
    },
    options::Options,
    shared::{NetworkSide, NumberKind, Range, Remote},
};

mod iter;
mod send;

struct ServerCtx {
    pub global_ctx: Ctx,
}

struct RecvCtx {
    pub jump_tbl: TVar,
    pub item_idx: usize,
}

impl RecvCtx {
    fn listen(&self, b: &mut Builder, listener: impl Into<Expr>) {
        b.assign_index(&self.jump_tbl, self.item_idx, listener.into());
    }
}

pub struct SendCtx<'a> {
    pub instance: TVar,
    pub item_idx: usize,
    pub idx_kind: NumberKind,
    pub ctx: &'a Ctx,
}

impl SendCtx<'_> {
    fn write_idx(&self, b: &mut Builder, ctx: &Ctx) {
        b.check(ctx, self.idx_kind.size());
        b.write_k(ctx, self.idx_kind.into(), self.item_idx);
    }

    fn send(&self, b: &mut Builder, player: impl Into<Expr>, ctx: &Ctx) {
        let buf = b.init(Expr::Global("buffer.create").call(vec![ctx.pos.expr()]));

        b.call(Expr::Global("buffer.copy").call(vec![
            buf.expr(),
            0.into(),
            ctx.buf.expr(),
            0.into(),
            ctx.pos.expr(),
        ]));

        b.call(
            self.instance
                .expr()
                .namecall("FireClient", vec![player.into(), buf.expr()]),
        );

        b.assign(&ctx.pos, 0);
    }

    fn load_ctx(&self, b: &mut Builder) -> Ctx {
        Ctx {
            buf: b.init(&self.ctx.buf),
            pos: b.init(&self.ctx.pos),
            len: b.init(&self.ctx.len),
        }
    }

    fn save_ctx(&self, b: &mut Builder, ctx: &Ctx) {
        b.assign(&self.ctx.buf, &ctx.buf);
        b.assign(&self.ctx.pos, &ctx.pos);
        b.assign(&self.ctx.len, &ctx.len);
    }
}

pub fn server(hir: &Hir) -> String {
    let mut b = Builder::default();
    let serverctx = ServerCtx {
        global_ctx: Ctx {
            buf: b.init(Expr::Global("buffer.create").call(vec![1024.into()])),
            pos: b.init(0),
            len: b.init(1024),
        },
    };

    self::buckets(&mut b, &serverctx, hir.buckets());

    let mut interface = String::new();
    self::interface(&mut interface, &hir.table).unwrap();

    format!(
        "{}{}{}::{}",
        include_str!("../../header.luau"),
        b.build(),
        include_str!("../../footer.luau"),
        interface
    )
}

fn buckets(b: &mut Builder, serverctx: &ServerCtx, buckets: Buckets) {
    for (remote, items) in buckets {
        self::remote(b, serverctx, &remote, &items);
    }
}

fn remote(b: &mut Builder, serverctx: &ServerCtx, remote: &Remote, items: &[(String, &Item)]) {
    let uuid = remote.uuid.to_string();

    let mut recv = Vec::new();
    let mut send = Vec::new();

    for (path, item) in items {
        match item {
            Item::Table(_) => unreachable!(),
            Item::Event(event) => match event.from {
                NetworkSide::Client => recv.push((path, item)),
                NetworkSide::Server => send.push((path, item)),
            },
        }
    }

    let recv_item_kind = Range::new(Some(1f64), Some(recv.len() as f64)).kind();
    let send_item_kind = Range::new(Some(1f64), Some(send.len() as f64)).kind();

    let mut recvctx = RecvCtx {
        jump_tbl: b.init(Expr::Table(vec![])),
        item_idx: 0,
    };

    let mut max_recv_size = Size::default();

    for (path, item) in recv {
        recvctx.item_idx += 1;

        match item {
            Item::Table(_) => unreachable!(),
            Item::Event(event) => iter::iter(b, &recvctx, path, event),
        }

        let size = match item {
            Item::Table(_) => unreachable!(),
            Item::Event(event) => event.data.size_of(),
        };

        if size > max_recv_size {
            max_recv_size = size;
        }
    }

    let mut sendctx = SendCtx {
        instance: b.init(Expr::Global("folder").namecall("WaitForChild", vec![Expr::from(uuid)])),
        item_idx: 0,
        idx_kind: send_item_kind,
        ctx: &serverctx.global_ctx,
    };

    for (path, item) in send {
        sendctx.item_idx += 1;

        match item {
            Item::Table(_) => unreachable!(),
            Item::Event(event) => send::event(b, &sendctx, path, event),
        }
    }

    let listener = b.function(|b, [plr, buf]| {
        let ctx = Ctx {
            buf: b.init(buf.expr()),
            pos: b.init(0),
            len: b.init(Expr::Global("buffer.len").call(vec![buf.expr()])),
        };

        if let Some(max) = max_recv_size.max {
            b.assert(ctx.len.expr().le(max), "packet too large");
        }

        let idx = b.read_k(&ctx, recv_item_kind.into());
        b.call(recvctx.jump_tbl.expr().index(&idx).call(vec![
            plr.expr(),
            ctx.buf.expr(),
            ctx.pos.expr(),
            ctx.len.expr(),
        ]));
    });

    b.call(
        sendctx
            .instance
            .expr()
            .index("OnServerEvent")
            .namecall("Connect", vec![listener.expr()]),
    );
}

fn interface(s: &mut String, table: &Table) -> std::fmt::Result {
    use std::fmt::Write;

    write!(s, "{{ ")?;

    for (name, item) in &table.items {
        write!(s, "{name}: ")?;

        match item {
            Item::Table(table) => interface(s, table)?,
            Item::Event(event) => match event.from {
                NetworkSide::Client => self::iter::interface(s, event)?,
                NetworkSide::Server => self::send::interface(s, event)?,
            },
        }

        write!(s, ", ")?;
    }

    write!(s, " }}")
}

fn ser(opts: &Options) -> Ser {
    Ser {
        opts: opts.clone(),
        native: true,
    }
}

fn des(opts: &Options) -> Des {
    Des {
        opts: opts.clone(),
        native: true,
        check: false,
    }
}
