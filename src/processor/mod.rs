use std::{cell::RefCell, collections::{HashMap, HashSet}, pin::Pin, sync::atomic::{AtomicBool, Ordering}, task::{Context, Poll, Waker}, thread::JoinHandle};

use aion_ecs::prelude::{GetShared, GetUnique};
use execution_graph::prelude::{Graph, Link, Node};
use hecs::Entity;

use crate::prelude::{DumbWaker, ExecuteSystemResult, SystemId, SystemStatus, sync::{Arc, ArcRwLockWriteGuard, RawRwLock, RwLock, Mutex}};

use aion_program::prelude::{AccessBuilder, ProgramId, ProgramRegistry};

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
    /// 
    /// To prevent indefinite blocking ensure the `World` resource is "public"
    /// 
    /// i.e take ownership of the `World` and then `whitelist` it
    /// 
    /// Best advice is to mock each system before putting it in `queue`
    /// 
    /// via Async/Sync System::check_accesses
    /// 
    /// This means that even if a system is blocked for the duration of this function (via a resolve which isn't dropped), 
    /// it will never be queued and so won't cause indefinite blocking
    pub fn process_blocking(
        system_queue: HashSet<SystemId>,
        links: Vec<Link<SystemId>>,
        main_thread_systems: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        let statuses = Arc::new(system_queue.iter().map(|system_id| (system_id, AtomicBool::new(true))).collect::<HashMap<_, _>>());

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
                let statuses = Arc::clone(&statuses);
        
                let results_tx = results_tx.clone();
                
                let runtime = tokio::runtime::Handle::current();

                scope.spawn(move |_| {
                    
                    let thread_label = format!("Thread: {current_thread}");
            
                    LABEL.with(|label| {
                        label.replace(Some(thread_label));
                    });
            
                    let results = runtime.block_on(
                        Self::execute_graph(&graph, &program_registry, &thread_blacklist, &program_details_map, &statuses)
                    );
            
                    let _ = results_tx.send(results);
                });
            }

            let main_thread_label = format!("Main Thread");

            LABEL.with(|label| {
                label.replace(Some(main_thread_label));
            });

            let main_results = tokio::runtime::Handle::current().block_on(
                Self::execute_graph(&graph, program_registry, &Arc::new(RwLock::new(HashSet::default())), &program_details_map, &statuses)
            );

            let _ = results_tx.send(main_results);
        });

        drop(results_tx);

        results_rx.iter().flat_map(|results| results).collect()
    }

    async fn execute_graph(
        graph: &RwLock<Graph<SystemId>>,
        program_registry: &Arc<ProgramRegistry>,
        blacklisted_systems: &RwLock<HashSet<SystemId>>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
        statuses: &HashMap<&SystemId, AtomicBool>
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
                        program_details_map,
                        statuses
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
                        
                        let get_status = program_registry.resolve_async::<GetShared<Mutex<SystemStatus>>>(Some(*system_entity), vec![program_access_builder]);
                        let status = match get_status {
                            Ok(Ok(get_status)) => Some(get_status),
                            Ok(Err(future_get_shared)) => Some(future_get_shared.await),
                            Err(_) => None,
                        };

                        if let Some(status) = status {
                            let status = status.get_shared();
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
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
        statuses: &HashMap<&SystemId, AtomicBool>,
    ) -> Option<ExecuteSystemResult<'a>> {
        let (program_id, system_entity) = node.data();

        let default_program_details = ProgramDetails::default();
        let program_details = program_details_map.get(program_id).or(Some(&default_program_details)).unwrap();

        match Self::run_system(program_registry, program_details, *system_entity, &(program_id.clone(), *system_entity), statuses).await {
            finished @ Some(ExecuteSystemResult::Final(_)) => {
                node.complete();

                finished
            },
            pending @ Some(ExecuteSystemResult::Pending(_)) => {
                node.make_pending();

                pending
            },
            None => None,
        }
    }

    async fn run_system<'a>(
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
                    let access_builders = program_registry.resolve_async::<GetShared<Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);
                    
                    // clone for no world dependency
                    let access_builders = match access_builders {
                        Ok(Ok(access_builders)) => (*access_builders.get_shared()).clone(),
                        Ok(Err(future_access_builders)) => (*future_access_builders.await.get_shared()).clone(),
                        Err(_) => vec![]
                    };

                    let result = Self::execute_system(
                        system,
                        program_registry,
                        program_details,
                        system_entity,
                        &access_builders
                    ).await;

                    match result {
                        Ok(Ok(system_result)) => {
                            return Some(ExecuteSystemResult::Final(system_result))
                        },
                        Err(task) => {        
                            return Some(ExecuteSystemResult::Pending(task))
                        },
                        Ok(Err(_system_error)) => {
                            status.store(true, Ordering::SeqCst);
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

        let prepared_system = program_registry.resolve_async::<GetUnique<System>>(Some(system_entity), vec![program_access_builder]);

        let system = match prepared_system {
            Ok(Ok(system)) => Some(system),
            Ok(Err(future_system)) => Some(future_system.await),
            // means the "submission" either 
            // 1. didn't have the correct input- should be unreachable! since we give the `program_access_builder`
            // 2. the resource couldn't be resolved because there wasn't enough inputs- unreachable! if 1. is unreachable!
            Err(_) => unreachable!(),
        };

        if let Some(system) = system {
            system.get_unique().put_to_empty(SystemKind::Sync(sync_system))            
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

            let program_access_builder = program_details.clone().into_access_builder();

            let prepared_system = program_registry.resolve_async::<GetUnique<System>>(Some(system_entity), vec![program_access_builder]);

            let system = match prepared_system {
                Ok(Ok(system)) => Some(system),
                Ok(Err(future_system)) => Some(future_system.await),
                // means the "submission" either 
                // 1. didn't have the correct input- should be unreachable! since we give the `program_access_builder`
                // 2. the resource couldn't be resolved because there wasn't enough inputs- unreachable! if 1. is unreachable!
                Err(_) => unreachable!(),
            };

            if let Some(system) = system {
                system.get_unique().put_to_empty(SystemKind::Async(async_system))            
            } 

            result
        })
    }

    /// REQUIRES A TOKIO RUNTIME WITH `.enter()`
    /// 
    /// cancel unsafe bc some `System`s may be unrecoverable
    /// 
    /// To prevent indefinite blocking ensure the `World` resource is "public"
    /// 
    /// i.e take ownership of the `World` and then `whitelist` it
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
                            let runtime = tokio::runtime::Handle::current();
                            let join_handle = std::thread::spawn(move || {
                                let thread_work = async {
                                    let access_builders = {
                                        let access_builders = program_registry.resolve_async::<GetShared<Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);
                                        match access_builders {
                                            Ok(Ok(access_builders)) => (*access_builders.get_shared()).clone(),
                                            Ok(Err(future_access_builders)) => (*future_access_builders.await.get_shared()).clone(),
                                            Err(_) => vec![]
                                        }
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
                                    let access_builders = program_registry.resolve_async::<GetShared<Vec<AccessBuilder>>>(Some(system_entity), vec![program_access_builder]);

                                    match access_builders {
                                        Ok(Ok(access_builders)) => (*access_builders.get_shared()).clone(),
                                        Ok(Err(future_access_builders)) => (*future_access_builders.await.get_shared()).clone(),
                                        Err(_) => vec![]
                                    }
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
                },
                Some(None) => { /* Could not find system */},
                None => { /* Resolve failed, should be unreachable! */ }
            }
        }
    
        (sync_handles, async_handles)
    }
}