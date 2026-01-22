use aion_program::prelude::{ResourceAccess, ResourceId, ResourceKeyId, ResourceReserverId};

pub struct SystemAccessDescriptor {
    inner: Vec<(ResourceId, ResourceAccess, Option<ResourceReserverId>, Option<ResourceKeyId>)>
}

impl SystemAccessDescriptor {
    pub fn get_accesses(&self) -> &Vec<(ResourceId, ResourceAccess, Option<ResourceReserverId>, Option<ResourceKeyId>)> {
        &self.inner
    }
}