use crate::{
    hir::{Event, Size},
    mir::{
        Expr,
        builder::{Builder, Ctx},
        serdes::Serdes,
        server::{RecvCtx, des},
    },
};

pub fn iter(b: &mut Builder, recvctx: &RecvCtx, path: &str, event: &Event) {
    match event.data.len() {
        0 => self::iter_0data(b, recvctx, path, event),
        _ => self::iter_ndata(b, recvctx, path, event),
    }
}

fn iter_0data(b: &mut Builder, recvctx: &RecvCtx, path: &str, event: &Event) {
    let queue = b.init(Expr::Table(vec![]));

    let listener = b.function(|b, [plr]| {
        b.call(Expr::Global("table.insert").call(vec![queue.expr(), plr.expr()]));
    });

    recvctx.listen(b, &listener);

    let iter = b.function(|b, []| {
        let captured = b.init(queue.expr());
        b.assign(&queue, Expr::Table(vec![]));
        b.ret(vec![captured.expr()]);
    });

    b.export(path, event.opts.casing.fmt("iter"), &iter)
}

fn iter_ndata(b: &mut Builder, recvctx: &RecvCtx, path: &str, event: &Event) {
    let queue = b.init(Expr::Table(vec![]));

    let des = des(&event.opts);
    let data = event
        .data
        .iter()
        .map(|ty| ty.des(b, &des))
        .collect::<Vec<_>>();

    let listener = b.function(|b, [plr, buf, pos, len]| {
        let ctx = Ctx {
            buf: b.init(buf.expr()),
            pos: b.init(pos.expr()),
            len: b.init(len.expr()),
        };

        b.call(Expr::Global("table.insert").call(vec![queue.expr(), plr.expr()]));

        for des in data.into_iter() {
            let value = des(b, &ctx);
            b.call(Expr::Global("table.insert").call(vec![queue.expr(), value.expr()]));
        }
    });

    recvctx.listen(b, &listener);

    let next = b.function(|b, [captured, i]| {
        b.branch(
            captured.expr().index(i).eq(Expr::Nil).not(),
            |b| {
                let mut rets = vec![i.expr().add(event.data.len() + 1)];

                for j in 0..=event.data.len() {
                    rets.push(captured.expr().index(i.expr().add(j)));
                }

                b.ret(rets);
            },
            |_| {},
        );
    });

    let iter = b.function(|b, []| {
        let captured = b.init(queue.expr());
        b.assign(&queue, Expr::Table(vec![]));
        b.ret(vec![next.expr(), captured.expr(), 1.into()]);
    });

    b.export(path, event.opts.casing.fmt("iter"), &iter)
}

pub fn interface(s: &mut String, event: &Event) -> std::fmt::Result {
    use std::fmt::Write;

    match event.data.len() {
        0 => write!(s, "{{ iter: () -> {{Player}} }}"),
        _ => {
            let args = event
                .data
                .iter()
                .map(|ty| format!(", {ty}"))
                .collect::<String>();

            write!(
                s,
                "{{ iter: () -> (({{ any }}, number?) -> (number?, Player{args}), {{ any }}) }}"
            )
        }
    }
}
