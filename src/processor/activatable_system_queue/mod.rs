use std::{collections::HashMap, sync::Arc};

use aion_program::prelude::ProgramRegistry;
use aion_system::prelude::StoredSystemMetadata;
use execution_graph::prelude::{Graph, Link};

use crate::prelude::{SystemCell, SystemId, SystemQueue};

pub struct ActivatableSystemQueue {
    activatable_systems: HashMap<SystemId, (SystemCell, StoredSystemMetadata)>
}

impl ActivatableSystemQueue {
    pub fn new(system_queue: SystemQueue, program_registry: &Arc<ProgramRegistry>) -> Self {
        let activatable_systems = system_queue.promote(program_registry).collect();
        Self { activatable_systems }
    }

    pub fn compile(&self, links: Vec<Link<SystemId>>) -> Graph<SystemId> {
        let world = self.activatable_systems
            .keys()
            .map(|(program_id, system_resource_id)| {
                ((*program_id).clone(), (*system_resource_id).clone())
            });

        Graph::new(world.collect(), links)
    }

    pub fn take_systems(self) -> HashMap<SystemId, (SystemCell, StoredSystemMetadata)> {
        self.activatable_systems
    }

    pub fn get_systems(&self) -> &HashMap<SystemId, (SystemCell, StoredSystemMetadata)> {
        &self.activatable_systems
    }
}