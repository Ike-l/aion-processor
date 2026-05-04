use std::collections::HashSet;

use aion_system::prelude::{StoredSystemMetadata};

use crate::prelude::{Invariant, CurrentSystemBlockers};

use aion_event::prelude::CurrentEvents;

pub struct SystemRegistry {
    registry: HashSet<StoredSystemMetadata>
}

impl SystemRegistry {
    pub fn enqueue(
        &self,
        invariants: &Vec<Invariant>,
        blockers: &CurrentSystemBlockers,
        events: &CurrentEvents,
    ) -> Vec<&StoredSystemMetadata> {
        self.registry
            .iter()
            .filter(|system_metadata| {
                blockers.blocks(system_metadata.resource_id())
            })
            .filter(|system_metadata| {
                invariants.iter().any(|invariant| {
                    match invariant {
                        Invariant::ReadOnly => system_metadata.is_readonly(),
                        Invariant::EventCondition => system_metadata.test_events(events),
                    }
                })
            }).collect()
    }
}