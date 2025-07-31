use crate::{
    hir::Event,
    mir::{builder::Builder, client::Client, serdes::Serdes},
};

impl Client {
    pub fn event_send(&self, b: &mut Builder, event: &Event) {
        match event.data.len() {
            0 => self.event_send_0data(b, event),
            _ => self.event_send_ndata(b, event),
        }
    }

    pub fn event_send_ndata(&self, b: &mut Builder, event: &Event) {
        let remote = self.remote(b, event);
        let n = event.data.len();

        let ser = event
            .data
            .iter()
            .map(|ty| ty.ser(b, &self.ser))
            .collect::<Vec<_>>();

        let send = b.function_n(n, |b, args| {
            b.stmt("pos = 0");

            for i in 0..n {
                let arg = &args[i];
                let ser = &ser[i];

                ser(b, arg.expr());
            }

            b.stmt("local out = buffer.create(pos)");
            b.stmt("buffer.copy(out, 0, buf, 0, pos)");
            b.stmt(format!("{remote}:FireServer(out)"));
        });

        self.export(b, &self.name("send"), &send);
    }

    pub fn event_send_0data(&self, b: &mut Builder, event: &Event) {
        let remote = self.remote(b, event);

        let send = b.function(|b, []| {
            b.stmt(format!("{remote}:FireServer()"));
        });

        self.export(b, &self.name("send"), &send);
    }
}
