use std::collections::HashMap;

use aion_system::prelude::{StoredSystemMetadata};

use crate::prelude::{Invariant, CurrentSystemBlockers, SystemQueue, SystemId};

use aion_event::prelude::CurrentEvents;

use aion_program::prelude::ProgramId;

pub mod system_queue;
pub mod invariant;
pub mod system_id;

pub struct SystemRegistry<'a> {
    registry: HashMap<(ProgramId, &'a SystemId), &'a StoredSystemMetadata>
}

impl<'a> SystemRegistry<'a> {
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