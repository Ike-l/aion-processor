pub mod system_result_builder;

use std::collections::HashSet;

use aion_event::prelude::Event;

pub struct SystemResult {
    spawn_events: HashSet<Event>,
    remove_self: bool,
}

impl Default for SystemResult {
    fn default() -> Self {
        Self::new(HashSet::new(), false)
    }
}

impl SystemResult {
    pub fn new(
        spawn_events: HashSet<Event>, 
        remove_self: bool
    ) -> Self {
        Self {
            spawn_events, remove_self
        }
    }

    pub fn with_event(&mut self, event: Event) {
        self.spawn_events.insert(event);
    }

    pub fn with_events(&mut self, events: impl Iterator<Item = Event>) {
        self.spawn_events.extend(events);
    }

    pub fn with_self_as(&mut self, remove_self: bool) {
        self.remove_self = remove_self
    }
}

