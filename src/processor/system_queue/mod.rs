use std::collections::HashMap;

use aion_system::prelude::{StoredSystemMetadata};
use aion_program::prelude::{ProgramId, ResourceId};
use execution_graph::prelude::{Graph, Link};

use crate::prelude::GraphIdentifier;

pub struct SystemQueue<'a> {
    systems: HashMap<(&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata>
}

impl<'a> SystemQueue<'a> {
    pub fn new(systems: impl Iterator<Item = ((&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata)>) -> Self {
        Self {
            systems: systems.collect()
        }
    }

    pub fn get(&self, key: &(&'a ProgramId, &'a ResourceId)) -> Option<&&StoredSystemMetadata> {
        self.systems.get(key)
    }

    pub fn compile(&self, links: Vec<Link<GraphIdentifier>>) -> Graph<GraphIdentifier> {
        let world = self.systems
            .keys()
            .map(|(program_id, system_resource_id)| {
                ((*program_id).clone(), (*system_resource_id).clone())
            });

        Graph::new(world.collect(), links)
    }

    pub fn systems(&self) -> &HashMap<(&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata> {
        &self.systems
    }
}