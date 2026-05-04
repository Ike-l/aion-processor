use std::collections::HashMap;

use aion_system::prelude::{StoredSystemMetadata};
use aion_program::prelude::{ProgramId};
use execution_graph::prelude::{Graph, Link};

use crate::prelude::{SystemId};

pub struct SystemQueue<'a> {
    systems: HashMap<&'a (ProgramId, SystemId), &'a StoredSystemMetadata>
}

impl<'a> SystemQueue<'a> {
    pub fn new(systems: impl Iterator<Item = (&'a (ProgramId, SystemId), &'a StoredSystemMetadata)>) -> Self {
        Self {
            systems: systems.collect()
        }
    }

    pub fn compile(&self, links: Vec<Link<(ProgramId, SystemId)>>) -> Graph<(ProgramId, SystemId)> {
        let world = self.systems
            .keys()
            .cloned()
            .cloned();

        Graph::new(world.collect(), links)
    }
}