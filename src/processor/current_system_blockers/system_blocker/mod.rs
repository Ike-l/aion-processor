use aion_program::prelude::{ResourceId};

pub struct SystemBlocker {
    blocks: ResourceId
}

impl SystemBlocker {
    pub fn blocks(&self, resource_id: &ResourceId) -> bool {
        self.blocks == *resource_id
    }
}