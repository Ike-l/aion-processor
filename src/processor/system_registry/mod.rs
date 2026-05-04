use std::collections::{HashMap, HashSet};

use aion_system::prelude::{StoredSystemMetadata};

use crate::prelude::{Invariant, CurrentSystemBlockers, SystemQueue};

use aion_event::prelude::CurrentEvents;

use aion_program::prelude::{ProgramId, ResourceId};

pub mod system_queue;
pub mod invariant;

pub struct SystemRegistry<'a> {
    registry: HashMap<(ProgramId, &'a ResourceId), &'a StoredSystemMetadata>
}

impl<'a> SystemRegistry<'a> {
    pub fn new(registries: HashMap<ProgramId, HashSet<&'a StoredSystemMetadata>>) -> Self {
        let mut registry = HashMap::new();
        for (program_id, systems) in registries {
            for system in systems {
                registry.insert((program_id.clone(), system.resource_id()), system);
            }            
        }

        Self { registry }
    }

    pub fn enqueue(
        &'a self,
        invariants: &Vec<Invariant>,
        blockers: &CurrentSystemBlockers,
        events: &CurrentEvents,
    ) -> SystemQueue<'a> {
        SystemQueue::new(self.registry
            .iter()
            .filter(|(_, system_metadata)| {
                blockers.blocks(system_metadata.resource_id())
            })
            .filter(|(_, system_metadata)| {
                invariants.iter().any(|invariant| {
                    match invariant {
                        Invariant::ReadOnly => system_metadata.is_readonly(),
                        Invariant::EventCondition => system_metadata.test_events(events),
                    }
                })
            })
            .map(|((program_id, system_id), system_metadata)| {
                ((program_id, *system_id), *system_metadata)
            })  
        )
    }
}