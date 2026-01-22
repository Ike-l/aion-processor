use std::collections::HashMap;

use aion_event::prelude::CurrentEvents;
use aion_program::prelude::{ResourceId, ResourceKeyId};

use crate::prelude::{Criteria, ProcessorOrdering, SystemId};

pub mod processor_ordering;
pub mod criteria;
pub mod system_id;

pub struct SystemMetadata {
    // parameters at `usize` gets key `ResourceKeyId`
    keys: HashMap<usize, ResourceKeyId>,
    system_id: SystemId,
    
    system_resource_id: ResourceId,
    system_resource_key_id: Option<ResourceKeyId>,

    ordering: ProcessorOrdering,
    criteria: Criteria,
}

impl SystemMetadata {
    pub fn get_keys(&self) -> &HashMap<usize, ResourceKeyId> {
        &self.keys
    }

    pub fn test(&self, current_events: &CurrentEvents) -> bool {
        self.criteria.test(current_events)
    }
}