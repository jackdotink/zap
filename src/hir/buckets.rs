use std::collections::HashMap;

use crate::{
    hir::{Hir, Item, Table},
    shared::Remote,
};

pub type Buckets<'hir> = HashMap<Remote, Vec<(String, &'hir Item)>>;

impl Hir {
    pub fn buckets(&self) -> Buckets {
        fn visit<'hir>(
            buckets: &mut HashMap<Remote, Vec<(String, &'hir Item)>>,
            table: &'hir Table,
            path: String,
        ) {
            for (name, item) in table.items.iter() {
                let path = format!("{path}.{name}");

                match item {
                    Item::Table(table) => visit(buckets, table, path),
                    Item::Event(event) => buckets
                        .entry(event.thru.clone())
                        .or_default()
                        .push((path, item)),
                }
            }
        }

        let mut buckets = HashMap::new();
        visit(&mut buckets, &self.table, String::new());

        buckets
    }
}
