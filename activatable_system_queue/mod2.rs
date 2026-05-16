use std::{collections::{HashMap, HashSet}, sync::Arc};

use aion_program::prelude::ProgramRegistry;
use aion_system::prelude::StoredSystemMetadata;
use execution_graph::prelude::{Graph, Link};

use crate::prelude::{SystemCell, SystemId, SystemQueue};

pub struct ActivatableSystemQueue {
    queue: HashSet<SystemId>
}

impl ActivatableSystemQueue {
    pub fn new(queue: impl Iterator<Item = SystemId>) -> Self {
        Self { queue }
    }

    pub fn compile(&self, links: Vec<Link<SystemId>>) -> Graph<SystemId> {
        let world = self.activatable_systems.clone();

        Graph::new(world, links)
    }

    pub fn take_systems(self) -> HashSet<SystemId> {
        self.activatable_systems
    }

    pub fn get_systems(&self) -> &HashSet<SystemId> {
        &self.activatable_systems
    }
}