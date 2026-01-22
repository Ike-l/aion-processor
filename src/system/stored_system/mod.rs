use crate::prelude::{SystemAccessDescriptor, SystemResourceDescriptor};

pub mod system_resource_descriptor;
pub mod system_access_descriptor;

pub struct StoredSystem {
    resource_descriptor: SystemResourceDescriptor,
    access_descriptor: SystemAccessDescriptor,
}
// Vec is ordered by parameter
// i.e index 0 is for parameter 0 (from left) of function signature

impl StoredSystem {
    pub fn get_resource_descriptor(&self) -> &SystemResourceDescriptor {
        &self.resource_descriptor
    }

    pub fn get_access_descriptor(&self) -> &SystemAccessDescriptor {
        &self.access_descriptor
    }
}