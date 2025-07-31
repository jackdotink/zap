use crate::{
    hir::Event,
    mir::{builder::Builder, serdes::Serdes, server::Server},
};

impl Server {
    pub fn event_send(&self, b: &mut Builder, event: &Event) {
        match event.data.len() {
            0 => self.event_send_0data(b, event),
            _ => self.event_send_ndata(b, event),
        }
    }

    fn event_send_ndata(&self, b: &mut Builder, event: &Event) {
        let remote = self.remote(b, event);
        let n = event.data.len();

        let ser = event
            .data
            .iter()
            .map(|ty| ty.ser(b, &self.ser))
            .collect::<Vec<_>>();

        let send = b.function_n(n, |b, args| {
            let player = &args[0];
            b.stmt("pos = 0");

            for i in 0..n {
                let arg = &args[i + 1];
                let ser = &ser[i];

                ser(b, arg.expr());
            }

            b.stmt("local out = buffer.create(pos)");
            b.stmt("buffer.copy(out, 0, buf, 0, pos)");
            b.stmt(format!("{remote}:FireClient({player}, out)"));
        });

        self.export(b, &self.name("send"), &send);
    }

    fn event_send_0data(&self, b: &mut Builder, event: &Event) {
        let remote = self.remote(b, event);

        let send = b.function(|b, [player]| {
            b.stmt(format!("{remote}:FireClient({player})"));
        });

        self.export(b, &self.name("send"), &send);
    }
}
