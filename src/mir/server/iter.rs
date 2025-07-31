use crate::{
    hir::Event,
    mir::{
        Expr,
        builder::{Builder, InitVar},
        serdes::Serdes,
        server::Server,
    },
};

impl Server {
    pub fn event_recv_iter(&self, b: &mut Builder, event: &Event) {
        match event.data.len() {
            0 => self.event_recv_iter_0data(b, event),
            _ => self.event_recv_iter_ndata(b, event),
        }
    }

    fn event_recv_iter_ndata(&self, b: &mut Builder, event: &Event) {
        let queue = self.event_recv_queue_ndata(b, event);
        let n = event.data.len();

        let iter = b.function(|b, []| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table(vec![]));

            let next = b.function(|b, [captured, i]| {
                b.branch(
                    captured.expr().index(i),
                    |b| {
                        let rets = (0..=n)
                            .map(|j| format!(", {captured}[{i} + {j}]"))
                            .collect::<String>();

                        b.stmt(format!("return {i} + {n}{rets}"));
                    },
                    |_| {},
                )
            });

            b.stmt(format!("return {next}, {captured}, 1"));
        });

        self.export(b, &self.name("iter"), &iter);
    }

    fn event_recv_iter_0data(&self, b: &mut Builder, event: &Event) {
        let queue = self.event_recv_queue_0data(b, event);

        let iter = b.function(|b, []| {
            let captured = b.expr(&queue);
            b.assign(&queue, Expr::Table(vec![]));
            b.stmt(format!("return {captured}"));
        });

        self.export(b, &self.name("iter"), &iter);
    }

    fn event_recv_queue_ndata(&self, b: &mut Builder, event: &Event) -> InitVar {
        let remote = self.remote(b, event);
        let queue = b.expr(Expr::Table(vec![]));

        let des = event
            .data
            .iter()
            .map(|ty| ty.des(b, &self.des))
            .collect::<Vec<_>>();

        let on_event = b.function(|b, [player, buf]| {
            b.stmt(format!("local buf, pos = {buf}, 0"));
            b.stmt(format!("table.insert({queue}, {player})"));

            for des in des {
                let value = des(b);
                b.stmt(format!("table.insert({queue}, {value})"));
            }
        });

        b.stmt(format!(
            "{remote}.OnServerEvent:Connect(function(player, buf) pcall({on_event}, player, buf) end)"
        ));

        queue
    }

    fn event_recv_queue_0data(&self, b: &mut Builder, event: &Event) -> InitVar {
        let remote = self.remote(b, event);
        let queue = b.expr(Expr::Table(vec![]));

        let on_event = b.function(|b, [player]| {
            b.stmt(format!("table.insert({queue}, {player})"));
        });

        b.stmt(format!("{remote}.OnServerEvent:Connect({on_event})"));

        queue
    }
}
