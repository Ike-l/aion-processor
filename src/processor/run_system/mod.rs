use std::{collections::HashMap, sync::atomic::{AtomicBool, Ordering}};

use aion_ecs::prelude::{GetOwned, GetUnique};
use aion_program::prelude::{AccessBuilder, ProgramRegistry};
use aion_system::prelude::{ProgramDetails, System};
use hecs::Entity;

use crate::prelude::{ExecuteSystemResult, Processor, SystemId, sync::Arc, ExecuteSystem};

pub struct RunSystem;

impl Processor<RunSystem> {
    pub async fn run_system<'a>(
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        system_entity: Entity,
        system_id: &SystemId,
        statuses: &HashMap<&SystemId, AtomicBool>,
    ) -> Option<ExecuteSystemResult<'a>> {
        let program_access_builder = program_details.clone().into_access_builder();

        let status = statuses.get(&system_id).expect("Statuses should contain ALL system ids (to be executed)");
        if status.swap(false, Ordering::SeqCst) {
            let system = {
                let mut system = program_registry.resolve_async::<GetUnique<System>>(Some(system_entity), vec![program_access_builder.clone()]);
                match system.as_mut() {
                    Ok(Ok(system)) => Some(system.get_unique().take_system()),
                    Ok(Err(future_system)) => Some(future_system.await.get_unique().take_system()),
                    Err(_) => None,
                }
            };

            match system {
                Some(Some(system)) => {
                    // clone for no world dependency
                    let access_builders = program_registry.resolve_async::<GetOwned<Vec<AccessBuilder>, Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);
                    
                    let access_builders = match access_builders {
                        Ok(Ok(access_builders)) => access_builders.item,
                        Ok(Err(future_access_builders)) => future_access_builders.await.item,
                        Err(_) => vec![]
                    };

                    let result = Processor::<ExecuteSystem>::execute_system(
                        system,
                        program_registry,
                        program_details,
                        system_entity,
                        &access_builders
                    ).await;

                    match result {
                        Ok(Ok(Ok(system_result))) => {
                            return Some(ExecuteSystemResult::Final(system_result))
                        },
                        Ok(Ok(Err(_system_error))) => {
                            status.store(true, Ordering::SeqCst);
                        },
                        Ok(Err(panic_error)) => {
                            return Some(ExecuteSystemResult::Panicked(panic_error))
                        }
                        Err(task) => {        
                            return Some(ExecuteSystemResult::Pending(task))
                        },
                    }
                },
                // means someone has taken system before so dont try again
                // unreachable!()?
                Some(None) => {},
                // resolve fails: try again
                None => {
                    status.store(true, Ordering::SeqCst)
                },
            }
        }

        None
    }

}