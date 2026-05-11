use std::{collections::HashMap, sync::Arc};

use aion_system::prelude::{StoredSystem, StoredSystemMetadata};
use aion_program::prelude::{AccessBuilder, ProgramId, ProgramRegistry, ResourceId};

use crate::prelude::{SystemCell, SystemId, Unique};

pub struct SystemQueue<'a> {
    systems: HashMap<(&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata>
}

impl<'a> SystemQueue<'a> {
    pub fn new(systems: impl Iterator<Item = ((&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata)>) -> Self {
        Self {
            systems: systems.collect()
        }
    }

    pub fn get(&self, key: &(&'a ProgramId, &'a ResourceId)) -> Option<&&StoredSystemMetadata> {
        self.systems.get(key)
    }

    pub fn promote(self, program_registry: &Arc<ProgramRegistry>) -> impl Iterator<Item = (SystemId, (SystemCell, StoredSystemMetadata))> {
        self.systems.into_iter().filter_map(|((program_id, system_resource_id), system_metadata)| {
            let prompted_access = AccessBuilder {
                program_id: Some(program_id.clone()),
                program_password: system_metadata.system_program_password().clone(),
                user_details: system_metadata.user_details().clone(),
                resource_id: Some((*system_resource_id).clone()),
                resource_access: None,
                resource_password: None,
            };

           match program_registry.resolve::<Unique<StoredSystem>>(vec![prompted_access]) {
                Ok(Ok(mut stored_system)) => {
                    let system = stored_system.as_mut();

                    let (auto_access_builder, manual_access_builders) = system_metadata.get_access_builders((*program_id).clone());
                    let manual_access_builders = manual_access_builders.into_iter().map(|access_builder| access_builder).collect();
                    if !system.can_run(program_registry, &auto_access_builder, manual_access_builders) {
                        return None
                    }

                    /*
                        Need these Cells specifically for Async functions because
                        It could be multi-threaded 
                        We need unique access to the system and
                        If we were to use a guard to get the unique access,
                        When saving the async task/future to poll later,
                        It would need to be lifted out of the lifetime of the guard
                        We can not preemptively store the systems at the same level of the container because threads steal work and storing them would effectively cancel that out
                        So instead using a Cell we can get the unique access and store it
                    */

                    let system = system.take_system()?;
                    let system_cell = SystemCell::new(system);
                    Some((((*program_id).clone(), (*system_resource_id).clone()), (system_cell, (*system_metadata).clone())))
                },
                _ => None
            }
        })
    }   

    pub fn systems(&self) -> &HashMap<(&'a ProgramId, &'a ResourceId), &'a StoredSystemMetadata> {
        &self.systems
    }
}