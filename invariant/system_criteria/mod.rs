use std::fmt::Debug;

use aion_event::prelude::CurrentEvents;

pub struct SystemCriteria {
    activator: Box<dyn Fn(&CurrentEvents) -> bool + Send + Sync>
}

impl Debug for SystemCriteria {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "System Criteria")
    }
}

impl SystemCriteria {
    pub fn new(criteria: impl Fn(&CurrentEvents) -> bool + Send + Sync + 'static) -> Self {
        Self {
            activator: Box::new(criteria)
        }
    }

    pub fn test(&self, events: &CurrentEvents) -> bool {
        (self.activator)(events)
    }
}