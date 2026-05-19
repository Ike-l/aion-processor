use std::{cell::RefCell, collections::{HashMap, HashSet}, pin::Pin, task::{Context, Poll, Waker}, thread::JoinHandle};

use aion_ecs::prelude::World;
use execution_graph::prelude::{Graph, Link, Node};
use hecs::Entity;

use crate::prelude::{DumbWaker, ExecuteSystemResult, SystemId, SystemStatus, sync::{Arc, ArcRwLockWriteGuard, RawRwLock, RwLock, Mutex}};

use aion_program::prelude::{AccessBuilder, ProgramId, ProgramRegistry, Shared};

use aion_system::{prelude::{AsyncSystem, ProgramDetails, SyncSystem, System, SystemError, SystemResult}, system::SystemKind};

pub mod waker;
pub mod execute_system_result;
pub mod system_status;

thread_local! {
    static LABEL: RefCell<Option<String>> = RefCell::new(None);
}

pub struct Processor;

// TODO:
// Check if i can avoid cloning/storing system metadata and just fetch when i need? (performance implications, clone (and having clone on the metadata) vs fetch?)

impl Processor {
    // will keep trying to run all systems until they are done- so can be blocked if there is conflicting access
    // from a holder outside of the function

    // if there are systems which depend on a main thread system and there are no threads allocated
    // then will block infinitely

    // so go through the system queue and call check?
    /// REQUIRES A TOKIO RUNTIME WITH `.enter()`
    pub fn process_blocking(
        system_queue: HashSet<SystemId>,
        links: Vec<Link<SystemId>>,
        main_thread_systems: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        let graph = Arc::new(RwLock::new(Graph::new(system_queue.clone(), links)));

        let thread_blacklist = Arc::new(RwLock::new(main_thread_systems));
        
        let program_details_map = Arc::new(program_details.into_iter().map(|program_details| {
            (program_details.get_system_program().clone().expect("Global Program shouldn't have an 'owner'"), program_details)
        }).collect::<HashMap<_, _>>());

        let (results_tx, results_rx) = std::sync::mpsc::channel();

        let thread_count = 1;

        rayon::scope(|scope| {
            for current_thread in 0..thread_count {
                let program_registry = Arc::clone(program_registry);
        
                let graph = Arc::clone(&graph);
                let program_details_map = Arc::clone(&program_details_map);
                let thread_blacklist = Arc::clone(&thread_blacklist);
        
                let results_tx = results_tx.clone();
                
                let runtime = tokio::runtime::Handle::current();

                scope.spawn(move |_| {
                    
                    let thread_label = format!("Thread: {current_thread}");
            
                    LABEL.with(|label| {
                        label.replace(Some(thread_label));
                    });
            
                    let results = runtime.block_on(
                        Self::execute_graph(&graph, &program_registry, &thread_blacklist, &program_details_map)
                    );
            
                    let _ = results_tx.send(results);
                });
            }

            let main_thread_label = format!("Main Thread");

            LABEL.with(|label| {
                label.replace(Some(main_thread_label));
            });

            let main_results = tokio::runtime::Handle::current().block_on(
                Self::execute_graph(&graph, program_registry, &Arc::new(RwLock::new(HashSet::default())), &program_details_map)
            );

            let _ = results_tx.send(main_results);
        });

        drop(results_tx);

        results_rx.iter().flat_map(|results| results).collect()
    }

    async fn execute_graph(
        graph: &RwLock<Graph<SystemId>>,
        program_registry: &Arc<ProgramRegistry>,
        blacklisted_systems: &Arc<RwLock<HashSet<SystemId>>>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        let mut results = HashMap::new();

        let waker = Waker::from(Arc::new(DumbWaker));
        let mut context = Context::from_waker(&waker);
        let mut tasks: Vec<_> = Vec::new();
        
        while !graph.read().is_finished() {
            while let Some(leaf) = graph.read().find_leaves().pop() {
                if let Some(mut leaf) = leaf.try_write_arc() {
                    assert!(leaf.is_ready());

                    if blacklisted_systems.read().contains(leaf.data()) {
                        continue
                    }

                    let result = Self::run_node(
                        &mut leaf, 
                        program_registry,
                        program_details_map
                    ).await;

                    match result {
                        Some(ExecuteSystemResult::Final(system_result)) => {
                            results.insert(leaf.data().clone(), system_result);
                        },
                        Some(ExecuteSystemResult::Pending(task)) => {
                            tasks.push((task, ArcRwLockWriteGuard::into_arc(leaf)));
                        },
                        // Will try again later
                        None => (),
                    }
                }
            }

            let mut continuing_tasks = Vec::new();
            for (mut task, node) in tasks.drain(..) {
                let mut done = false;
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(result) => {     
                        let mut node = node.write();
                        let (program_id, system_entity) = node.data();

                        let program_details = program_details_map.get(program_id).cloned().unwrap_or_default();

                        let program_access_builder = program_details.into_access_builder();
                        
                        let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder]);
                        
                        let world = match world {
                            Ok(Ok(world)) => Some(world),
                            Ok(Err(future_world)) => Some(future_world.await),
                            Err(_) => None 
                        };

                        if let Some(world) = world {
                            let prepared_status = world.prepare_get_shared::<&Mutex<SystemStatus>>(*system_entity);
                            if let Some(status) = prepared_status {
                                let status = status.get(&world);
                                let mut status = status.lock();
                                match result {
                                    Ok(result) => {
                                        results.insert((program_id.clone(), *system_entity), result);
                                        *status = SystemStatus::Executed;
                                        node.complete();
                                    },
                                    Err(_system_error) => {
                                        *status = SystemStatus::Ready;                                
                                    },
                                }
                                
                                done = true;
                            }
                        }
                    },
                    Poll::Pending => {},
                }

                if !done {
                    continuing_tasks.push((task, node))
                }
            };

            tasks = continuing_tasks;
        }
    
        results
    }

    async fn run_node<'a>(
        node: &mut ArcRwLockWriteGuard<RawRwLock, Node<SystemId>>,
        program_registry: &Arc<ProgramRegistry>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>
    ) -> Option<ExecuteSystemResult<'a>> {
        let (program_id, system_entity) = node.data();

        let default_program_details = ProgramDetails::default();
        let program_details = program_details_map.get(program_id).or(Some(&default_program_details)).unwrap();

        match Self::run_system(program_registry, program_details, *system_entity).await {
            Some(ExecuteSystemResult::Final(system_result)) => {
                node.complete();

                Some(ExecuteSystemResult::Final(system_result))
            },
            Some(ExecuteSystemResult::Pending(task)) => {
                node.make_pending();

                Some(ExecuteSystemResult::Pending(task))
            },
            None => None,
        }
    }

    async fn run_system<'a>(
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        system_entity: Entity,
    ) -> Option<ExecuteSystemResult<'a>> {
        let program_access_builder = program_details.clone().into_access_builder();

        let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder]);
        let world = match world {
            Ok(Ok(world)) => Some(world),
            Ok(Err(future_world)) => Some(future_world.await),
            Err(_) => None 
        };

        if let Some(world) = world {
            let prepared_status = world.prepare_get_shared::<&Mutex<SystemStatus>>(system_entity);
            if let Some(status) = prepared_status {
                let status = status.get(&world);
                let system = match status.try_lock() {
                    Some(mut status) => {
                        match *status {
                            SystemStatus::Ready => {
                                let prepared_system = world.prepare_get_unique::<&mut System>(system_entity);
                                if let Some(system) = prepared_system {
                                    let mut system = system.get(&world);

                                    *status = SystemStatus::Executing;
                                    
                                    if let Some(system) = system.take_system() {
                                        Some(system)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            },
                            SystemStatus::Executing |
                            SystemStatus::Pending |
                            SystemStatus::Executed => { None /* Is benign */ },
                        }
                    },
                    None => None,
                };
            
                if let Some(system) = system {
                    let prepared_access_builders = world.prepare_get_shared::<&Vec<AccessBuilder>>(system_entity);
                    let access_builders = prepared_access_builders.and_then(|access_builders| Some(access_builders.get(&world)));
                    
                    let result = Self::execute_system(
                        system,
                        program_registry,
                        program_details,
                        system_entity,
                        access_builders.as_deref().unwrap_or(&&vec![])
                    ).await;
        
                    let mut status = status.lock();
                    match result {
                        Ok(Ok(system_result)) => {
                            *status = SystemStatus::Executed;
        
                            return Some(ExecuteSystemResult::Final(system_result))
                        },
                        Ok(Err(_system_error)) => {
                            *status = SystemStatus::Ready;
        
                            return None
                        },
                        Err(task) => {
                            *status = SystemStatus::Pending;
        
                            return Some(ExecuteSystemResult::Pending(task))
                        },
                    }
                }
            }
        }

        None
    }

    async fn execute_system<'a>(
        system_kind: SystemKind,
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        system_entity: Entity,
        access_builders: &Vec<AccessBuilder>
    ) -> Result<Result<Option<SystemResult>, SystemError>, Pin<Box<dyn Future<Output = Result<Option<SystemResult>, SystemError>> + Send + 'a>>> {
        match system_kind {
            SystemKind::Sync(sync_system) => {
                Ok(Self::execute_sync_system(
                    sync_system,
                    system_entity,
                    program_registry,
                    program_details,
                    access_builders
                ).await)
            },
            SystemKind::Async(async_system) => {
                let task = Self::execute_async_system(
                    async_system,
                    system_entity,
                    Arc::clone(program_registry),
                    program_details.clone(),
                    access_builders.clone(),
                );

                Err(task)
            },
        }
    }

    async fn execute_sync_system(
        mut sync_system: SyncSystem,
        system_entity: Entity,
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        access_builders: &Vec<AccessBuilder>
    ) -> Result<Option<SystemResult>, SystemError> {
        let result = sync_system.execute(
            system_entity,
            program_registry, 
            program_details,                 
            access_builders.iter().collect(),
        );

        let program_access_builder = program_details.clone().into_access_builder();

        let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder.clone()]);

        let world = match world {
            Ok(Ok(world)) => Some(world),
            Ok(Err(future_world)) => Some(future_world.await),
            Err(_) => None 
        };

        if let Some(world) = world {
            let prepared_status = world.prepare_get_shared::<&Mutex<SystemStatus>>(system_entity);
            if let Some(status) = prepared_status {
                let status = status.get(&world);
                let _status = status.lock();
                let prepared_system = world.prepare_get_unique::<&mut System>(system_entity);
                if let Some(system) = prepared_system {
                    let mut system = system.get(&world);
                    system.put_system(SystemKind::Sync(sync_system));
                }
            }
        }

        result
    }

    fn execute_async_system(
        mut async_system: AsyncSystem,
        system_entity: Entity,
        program_registry: Arc<ProgramRegistry>,
        program_details: ProgramDetails,
        access_builders: Vec<AccessBuilder>,
    ) -> Pin<Box<impl Future<Output = Result<Option<SystemResult>, SystemError>>>> {
        Box::pin(async move {   
            let result = async_system.execute(
                system_entity,
                Arc::clone(&program_registry),
                program_details.clone(),
                access_builders,
            ).await;


            let program_access_builder = program_details.into_access_builder();

            let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder.clone()]);

            let world = match world {
                Ok(Ok(world)) => Some(world),
                Ok(Err(future_world)) => Some(future_world.await),
                Err(_) => None 
            };

            if let Some(world) = world {
                let prepared_status = world.prepare_get_shared::<&Mutex<SystemStatus>>(system_entity);
                if let Some(status) = prepared_status {
                    let status = status.get(&world);

                    let _status = status.lock();
                    let prepared_system = world.prepare_get_unique::<&mut System>(system_entity);
                    if let Some(system) = prepared_system {
                        let mut system = system.get(&world);
                        system.put_system(SystemKind::Async(async_system));
                    }
                }
            } else {
                unreachable!()
            }

            result
        })
    }

    /// REQUIRES A TOKIO RUNTIME WITH `.enter()`
    pub async fn process_non_blocking(
        system_queue: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
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

            let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder.clone()]);

            let world = match world {
                Ok(Ok(world)) => Some(world),
                Ok(Err(future_world)) => Some(future_world.await),
                Err(_) => None 
            };

            if let Some(world) = world {
                let prepared_status = world.prepare_get_shared::<&Mutex<SystemStatus>>(system_entity);
                if let Some(status) = prepared_status {
                    let status = status.get(&world);
                    let mut status = status.lock();
                    *status = SystemStatus::Executing;

                    let prepared_system = world.prepare_get_unique::<&mut System>(system_entity);
                    if let Some(system) = prepared_system {
                        let mut system = system.get(&world);
                        let system = system.take_system();

                        // can refactor in future for the same approach as blocking (let system = if let ...)
                        drop(status);

                        if let Some(system) = system {
                            let program_registry = Arc::clone(program_registry);
                            match system {
                                SystemKind::Sync(sync_system) => {
                                    let runtime = tokio::runtime::Handle::current();
                                    let join_handle = std::thread::spawn(move || {
                                        let thread_work = async {
                                            let access_builders = {
                                                let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder]);
                                                let world = match world {
                                                    Ok(Ok(world)) => Some(world),
                                                    Ok(Err(future_world)) => Some(future_world.await),
                                                    Err(_) => None 
                                                };
    
                                                if let Some(world) = world {
                                                    let prepared_access_builders = world.prepare_get_shared::<&Vec<AccessBuilder>>(system_entity);
                                                    
                                                    let access_builders = prepared_access_builders.and_then(|access_builders| Some(access_builders.get(&world)));
                                                    
                                                    (*access_builders.as_deref().unwrap_or(&&vec![])).clone()
                                                } else { vec![] }
                                            };
                                                    
                                            Self::execute_sync_system(
                                                sync_system, 
                                                system_entity, 
                                                &program_registry, 
                                                &program_details, 
                                                &access_builders
                                            ).await
                                        };

                                        let result = runtime.block_on(thread_work);
                
                                        ((program_id, system_entity), result)
                                    });
                
                                    sync_handles.push(join_handle);
                                },
                                SystemKind::Async(async_system) => {
                                    let join_handle = tokio::spawn(async move {
                                        let access_builders = {
                                            let world = program_registry.resolve_async::<Shared<World>>(None, vec![program_access_builder]);
                                            let world = match world {
                                                Ok(Ok(world)) => Some(world),
                                                Ok(Err(future_world)) => Some(future_world.await),
                                                Err(_) => None 
                                            };

                                            if let Some(world) = world {
                                                let prepared_access_builders = world.prepare_get_shared::<&Vec<AccessBuilder>>(system_entity);
                                                
                                                let access_builders = prepared_access_builders.and_then(|access_builders| Some(access_builders.get(&world)));
                                                
                                                (*access_builders.as_deref().unwrap_or(&&vec![])).clone()
                                            } else { vec![] }
                                        };

                                        let result = Self::execute_async_system(
                                            async_system, 
                                            system_entity, 
                                            program_registry, 
                                            program_details, 
                                            access_builders
                                        ).await;
                
                                        ((program_id, system_entity), result)
                                    });
                
                                    async_handles.push(join_handle);
                                },
                            }
                        }
                    }
                }
            } else {
                unreachable!()
            }
        }
    
        (sync_handles, async_handles)
    }
}