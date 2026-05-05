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
                registry.insert((program_id.clone(), system.system_resource_id()), system);
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
                blockers.blocks(system_metadata.system_resource_id())
            })
            .filter(|((program_id, resource_id), _)| {
                invariants.iter().any(|invariant| {
                    match invariant {
                        Invariant::EventCondition(criteria_map) => {
                            criteria_map.get(&(program_id.clone(), (*resource_id).clone())).is_some_and(|criteria| criteria.test(events))
                        },
                    }
                })
            })
            .map(|((program_id, system_resource_id), system_metadata)| {
                ((program_id, *system_resource_id), *system_metadata)
            })  
        )
    }
}