use std::collections::HashMap;

use crate::prelude::{GraphIdentifier, SystemCriteria};

use aion_program::prelude::{ProgramId, ResourceId};
use aion_system::prelude::{StoredSystemMetadata};
use aion_event::prelude::CurrentEvents;

pub mod system_criteria;

pub enum Invariant<'a> {
    EventCondition{
        criteria_map: &'a HashMap<GraphIdentifier, SystemCriteria>,
        events: &'a CurrentEvents
    }
}

type InvariantFilter<'a> = ((ProgramId, &'a ResourceId), &'a StoredSystemMetadata);
impl<'a> Invariant<'a> {
    pub fn filter(
        &self, 
        filtering: impl Iterator<Item = InvariantFilter<'a>>
    ) -> impl Iterator<Item = InvariantFilter<'a>> {
        match self {
            Invariant::EventCondition {
                criteria_map ,
                events
            } => {
                filtering.filter(|((program_id, resource_id), _)| {
                    criteria_map.get(&(program_id.clone(), (*resource_id).clone())).is_some_and(|criteria| criteria.test(events))
                })
            },
        }
    }
}