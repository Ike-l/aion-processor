use std::{collections::{HashMap, HashSet}, thread::JoinHandle};

use aion_ecs::prelude::{GetOwned, GetUnique};
use aion_program::prelude::{AccessBuilder, ProgramRegistry};
use aion_system::prelude::{ProgramDetails, System, SystemError, SystemKind, SystemResult};

use crate::prelude::{ExecuteAsyncSystem, ExecuteSyncSystem, Processor, SystemId, sync::Arc};

pub struct NonBlocking;

impl Processor<NonBlocking> {
    /// cancel unsafe bc some `System`s may be unrecoverable
    /// 
    /// To prevent indefinite blocking ensure the `World` resource is "public"
    /// 
    /// i.e take ownership of the `World` and then `whitelist` it
    pub async fn process_non_blocking(
        system_queue: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
        handle: &tokio::runtime::Handle,
    ) -> (
        Vec<JoinHandle<(SystemId, Result<Option<SystemResult>, SystemError>)>>, 
        Vec<tokio::task::JoinHandle<(SystemId, Result<Option<SystemResult>, SystemError>)>>
    ) {        
        let program_details_map = Arc::new(program_details.into_iter().map(|program_details| {
            (program_details.get_system_program().clone().expect("Global Program shouldn't have an 'owner'"), program_details)
        }).collect::<HashMap<_, _>>());
        let default_program_details = ProgramDetails::default();

        let mut sync_handles = Vec::new();
        let mut async_handles = Vec::new();

        for (program_id, system_entity) in system_queue {
            let program_details = program_details_map.get(&program_id);

            let program_details = program_details.or(Some(&default_program_details)).unwrap().clone();
            let program_access_builder = program_details.clone().into_access_builder();

            let system = {
                let system = program_registry.resolve_async::<GetUnique<System>>(Some(system_entity), vec![program_access_builder.clone()]);
                match system {
                    Ok(Ok(system)) => Some(system.get_unique().take_system()),
                    Ok(Err(future_system)) => Some(future_system.await.get_unique().take_system()),
                    Err(_) => None
                }
            };

            match system {
                Some(Some(system)) => {
                    let program_registry = Arc::clone(program_registry);
                    match system {
                        SystemKind::Sync(sync_system) => {
                            let handle = handle.clone();
                            let join_handle = std::thread::spawn(move || {
                                let thread_work = async {
                                    let access_builders = {
                                        let access_builders = program_registry.resolve_async::<GetOwned<Vec<AccessBuilder>, Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);
                                        match access_builders {
                                            Ok(Ok(access_builders)) => access_builders.item,
                                            Ok(Err(future_access_builders)) => future_access_builders.await.item,
                                            Err(_) => vec![]
                                        }
                                    };
                                            
                                    Processor::<ExecuteSyncSystem>::execute_sync_system(
                                        sync_system, 
                                        system_entity, 
                                        &program_registry, 
                                        &program_details, 
                                        &access_builders
                                    ).await
                                };

                                let result = handle.block_on(thread_work);

                                let result = match result {
                                    Ok(system_result) => system_result,
                                    Err(panic_error) => Ok(Some(SystemResult::Panicked(panic_error)))
                                };

                                ((program_id, system_entity), result)
                            });
        
                            sync_handles.push(join_handle);
                        },
                        SystemKind::Async(async_system) => {
                            let join_handle = handle.spawn(async move {
                                let access_builders = {
                                    let access_builders = program_registry.resolve_async::<GetOwned<Vec<AccessBuilder>, Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);

                                    match access_builders {
                                        Ok(Ok(access_builders)) => access_builders.item,
                                        Ok(Err(future_access_builders)) => future_access_builders.await.item,
                                        Err(_) => vec![]
                                    }
                                };

                                let result = Processor::<ExecuteAsyncSystem>::execute_async_system(
                                    async_system, 
                                    system_entity, 
                                    program_registry, 
                                    program_details, 
                                    access_builders
                                ).await;

                                let result = match result {
                                    Ok(system_result) => system_result,
                                    Err(panic_error) => Ok(Some(SystemResult::Panicked(panic_error)))
                                };
        
                                ((program_id, system_entity), result)
                            });
        
                            async_handles.push(join_handle);
                        },
                    }
                },
                Some(None) => { /* Could not find system */},
                None => { /* Resolve failed, should be unreachable! */ }
            }
        }
    
        (sync_handles, async_handles)
    }
}