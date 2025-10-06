use crate::{
    hir::Event,
    mir::{
        Expr,
        builder::{Builder, Ctx, TVar},
        client::{RecvCtx, des},
        serdes::Serdes,
    },
};

pub fn iter(b: &mut Builder, recvctx: &RecvCtx, event: &Event) {
    match event.data.len() {
        0 => self::iter_0data(b, recvctx, event),
        1 => self::iter_1data(b, recvctx, event),
        _ => self::iter_ndata(b, recvctx, event),
    }
}

fn iter_0data(b: &mut Builder, recvctx: &RecvCtx, event: &Event) {
    let counter = b.init(0);

    let listener = b.function(|b, []| {
        b.assign(&counter, counter.expr().add(1));
    });

    recvctx.listen(b, &listener);

    let next = b.function(|b, [captured, i]| {
        b.branch(
            i.expr().lt(captured),
            |b| {
                b.ret(vec![i.expr().add(1)]);
            },
            |_| {},
        );
    });

    let iter = b.function(|b, []| {
        let captured = b.init(&counter);
        b.assign(&counter, 0);
        b.ret(vec![next.expr(), captured.expr(), 0.into()]);
    });

    b.export(&event.path, event.opts.casing.fmt("iter"), &iter)
}

fn queue(b: &mut Builder, recvctx: &RecvCtx, event: &Event) -> TVar {
    let queue = b.init(Expr::Table(vec![]));

    let des = des(&event.opts);
    let data = event
        .data
        .iter()
        .map(|ty| ty.des(b, &des))
        .collect::<Vec<_>>();

    let listener = b.function(|b, [buf, pos, len]| {
        let ctx = Ctx {
            buf: b.init(buf.expr()),
            pos: b.init(pos.expr()),
            len: b.init(len.expr()),
        };

        for des in data.into_iter() {
            let value = des(b, &ctx);
            b.call(Expr::Global("table.insert").call(vec![queue.expr(), value.expr()]));
        }
    });

    recvctx.listen(b, &listener);

    queue
}

fn iter_1data(b: &mut Builder, recvctx: &RecvCtx, event: &Event) {
    let queue = self::queue(b, recvctx, event);

    let iter = b.function(|b, []| {
        let captured = b.init(queue.expr());
        b.assign(&queue, Expr::Table(vec![]));
        b.ret(vec![captured.expr()]);
    });

    b.export(&event.path, event.opts.casing.fmt("iter"), &iter)
}

fn iter_ndata(b: &mut Builder, recvctx: &RecvCtx, event: &Event) {
    let queue = self::queue(b, recvctx, event);

    let next = b.function(|b, [captured, i]| {
        b.branch(
            captured.expr().index(i).eq(Expr::Nil).not(),
            |b| {
                let mut rets = vec![i.expr().add(event.data.len())];

                for j in 0..event.data.len() {
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

    b.export(&event.path, event.opts.casing.fmt("iter"), &iter)
}
