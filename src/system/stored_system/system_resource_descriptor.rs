use aion_program::prelude::ResourceId;

pub struct SystemResourceDescriptor {
    inner: Vec<ResourceId>
}

impl SystemResourceDescriptor {
    pub fn get_resources(&self) -> &Vec<ResourceId> {
        &self.inner
    }
}