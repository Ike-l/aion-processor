use std::collections::HashSet;

pub mod system_blocker;

use crate::prelude::SystemBlocker;

use aion_program::prelude::ResourceId;

pub struct CurrentSystemBlockers {
    blockers: HashSet<SystemBlocker>
}

impl CurrentSystemBlockers {
    pub fn blocks(&self, resource_id: &ResourceId) -> bool {
        self.blockers.iter().any(|blocker| blocker.blocks(resource_id))
    }
}