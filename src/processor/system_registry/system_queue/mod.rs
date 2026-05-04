use std::collections::HashMap;

use aion_system::prelude::StoredSystemMetadata;
use execution_graph::graph::{Graph, Link};

use crate::prelude::{SystemId};

pub struct SystemQueue<'a> {
    systems: HashMap<&'a SystemId, &'a StoredSystemMetadata>
}

impl<'a> SystemQueue<'a> {
    pub fn new(systems: impl Iterator<Item = (&'a SystemId, &'a StoredSystemMetadata)>) -> Self {
        Self {
            systems: systems.collect()
        }
    }

    pub fn compile(&self, links: Vec<Link<SystemId>>) -> Graph<SystemId> {
        let world = self.systems.keys().cloned().cloned();

        Graph::new(world.collect(), links)
    }
}