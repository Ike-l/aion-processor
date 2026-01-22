use aion_event::prelude::Event;

use crate::prelude::SystemResult;

#[derive(Default)]
pub struct SystemResultBuilder {
    current_system_result: SystemResult
}

impl SystemResultBuilder {
    pub fn with_event(mut self, event: Event) -> Self {
        self.current_system_result.with_event(event);
        self
    }
    pub fn with_events(mut self, events: impl Iterator<Item = Event>) -> Self {
        self.current_system_result.with_events(events);
        self
    }

    pub fn without_self(mut self) -> Self {
        self.current_system_result.with_self_as(true);
        self
    }

    pub fn with_self(mut self) -> Self {
        self.current_system_result.with_self_as(false);
        self
    }
}