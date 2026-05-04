use std::collections::HashMap;

use aion_system::prelude::{StoredSystemMetadata};
use aion_program::prelude::{ProgramId, ResourceId};
use execution_graph::prelude::{Graph, Link};

pub struct SystemQueue<'a> {
    systems: HashMap<(&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata>
}

impl<'a> SystemQueue<'a> {
    pub fn new(systems: impl Iterator<Item = ((&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata)>) -> Self {
        Self {
            systems: systems.collect()
        }
    }

    pub fn compile(&self, links: Vec<Link<(ProgramId, ResourceId)>>) -> Graph<(ProgramId, ResourceId)> {
        let world = self.systems
            .keys()
            .map(|(program_id, system_id)| {
                ((*program_id).clone(), (*system_id).clone())
            });

        Graph::new(world.collect(), links)
    }
}