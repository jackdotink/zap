use crate::{
    hir::Event,
    mir::{
        builder::Builder,
        client::{SendCtx, ser},
        serdes::Serdes,
    },
};

pub fn event(b: &mut Builder, sendctx: &SendCtx, event: &Event) {
    match event.data.len() {
        0 => self::send_0data(b, sendctx, event),
        _ => self::send_ndata(b, sendctx, event),
    }
}

fn send_0data(b: &mut Builder, sendctx: &SendCtx, event: &Event) {
    let send = b.function(|b, []| {
        let ctx = sendctx.load_ctx(b);
        sendctx.write_idx(b, &ctx);
        sendctx.send(b, &ctx);
        sendctx.save_ctx(b, &ctx);
    });

    b.export(&event.path, event.opts.casing.fmt("send"), &send);
}

fn send_ndata(b: &mut Builder, sendctx: &SendCtx, event: &Event) {
    let ser = ser(&event.opts);
    let data = event
        .data
        .iter()
        .map(|ty| ty.ser(b, &ser))
        .collect::<Vec<_>>();

    let send = b.function_n(data.len(), |b, args| {
        let ctx = sendctx.load_ctx(b);

        sendctx.write_idx(b, &ctx);
        for (ser, arg) in data.into_iter().zip(args) {
            ser(b, &ctx, arg.expr());
        }

        sendctx.send(b, &ctx);
        sendctx.save_ctx(b, &ctx);
    });

    b.export(&event.path, event.opts.casing.fmt("send"), &send);
}
