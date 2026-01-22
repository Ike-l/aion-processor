use std::sync::Arc;

use aion_event::prelude::CurrentEvents;

// trait aliasing 
// trait CriteriaFn = Fn(&CurrentEvents) -> bool + Send + Sync;

pub struct Criteria(Arc<dyn Fn(&CurrentEvents) -> bool + Send + Sync>);

impl Criteria {
    pub fn new<Criteria>(criteria: Criteria) -> Self 
        where Criteria: Fn(&CurrentEvents) -> bool + Send + Sync + 'static
    {
        Self(Arc::new(criteria))
    }

    pub fn test(&self, current_events: &CurrentEvents) -> bool {
        self.0(current_events)
    }
}