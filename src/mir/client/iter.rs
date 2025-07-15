use crate::{
    hir::Event,
    mir::{
        Expr,
        builder::{Builder, InitVar},
        client::Client,
        serdes::Serdes,
    },
};

impl Client {
    pub fn event_recv_iter(&self, b: &mut Builder, event: &Event) {
        match event.data.len() {
            0 => self.event_recv_iter_0data(b, event),
            1 => self.event_recv_iter_1data(b, event),
            _ => self.event_recv_iter_ndata(b, event),
        }
    }

    fn event_recv_iter_ndata(&self, b: &mut Builder, event: &Event) {
        let queue = self.event_recv_queue(b, event);
        let n = event.data.len();

        let iter = b.function(|b, []| {
            let captured = b.expr(queue.expr());
            b.assign(&queue, Expr::Table(vec![]));

            let next = b.function(|b, [captured, i]| {
                b.branch(
                    captured.expr().index(i),
                    |b| {
                        let rets = (0..n)
                            .map(|j| format!(", {captured}[{i} + {j}]"))
                            .collect::<String>();

                        b.stmt(format!("return {i} + {n}{rets}"));
                    },
                    |_| {},
                );
            });

            b.stmt(format!("return {next}, {captured}, 1"));
        });

        self.export(b, "iter", &iter);
    }

    fn event_recv_iter_1data(&self, b: &mut Builder, event: &Event) {
        let queue = self.event_recv_queue(b, event);

        let iter = b.function(|b, []| {
            let captured = b.expr(queue.expr());
            b.assign(&queue, Expr::Table(vec![]));
            b.stmt(format!("return {captured}"));
        });

        self.export(b, "iter", &iter);
    }

    fn event_recv_iter_0data(&self, b: &mut Builder, event: &Event) {
        let counter = self.event_recv_counter(b, event);

        let iter = b.function(|b, []| {
            let captured = b.expr(counter.expr());
            b.assign(&counter, 0);

            let next = b.function(|b, [captured, i]| {
                b.branch(
                    i.expr().lt(captured),
                    |b| {
                        b.stmt(format!("return {i} + 1"));
                    },
                    |_| {},
                );
            });

            b.stmt(format!("return {next}, {captured}, 0"));
        });

        self.export(b, "iter", &iter);
    }

    fn event_recv_queue(&self, b: &mut Builder, event: &Event) -> InitVar {
        let remote = self.remote(b, event);
        let queue = b.expr(Expr::Table(vec![]));

        let des = event
            .data
            .iter()
            .map(|ty| Box::new(ty.des(b, &self.des)))
            .collect::<Vec<_>>();

        let on_event = b.function(|b, [buf]| {
            b.stmt(format!("local buf, pos = {buf}, pos"));

            for des in des {
                let value = des(b);
                b.stmt(format!("table.insert({queue}, {value})"));
            }
        });

        b.stmt(format!("{remote}.OnClientEvent:Connect({on_event})"));
        queue
    }

    fn event_recv_counter(&self, b: &mut Builder, event: &Event) -> InitVar {
        let remote = self.remote(b, event);
        let counter = b.expr(0);

        let on_event = b.function(|b, []| {
            b.assign(&counter, counter.expr().add(1));
        });

        b.stmt(format!("{remote}.OnClientEvent:Connect({on_event})"));
        counter
    }
}
