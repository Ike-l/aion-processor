use std::{cell::RefCell, collections::{HashMap, HashSet}, pin::Pin, task::{Context, Poll, Waker}, thread::JoinHandle};

use aion_ecs::prelude::World;
use execution_graph::prelude::{Graph, Link, Node};
use hecs::Entity;
use tokio::runtime::Runtime;

use crate::prelude::{DumbWaker, ExecuteSystemResult, ProcessConfig, SystemId, SystemStatus, Unwinder, sync::{Arc, ArcRwLockWriteGuard, RawRwLock, RwLock, Mutex}};

use aion_program::prelude::{AccessBuilder, ProgramId, ProgramRegistry, Shared};

use aion_system::{prelude::{AsyncSystem, ProgramDetails, SyncSystem, System, SystemError, SystemResult}, system::SystemKind};

pub mod process_config;
pub mod waker;
pub mod unwinder;
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
    pub fn process_blocking(
        system_queue: HashSet<SystemId>,
        links: Vec<Link<SystemId>>,
        main_thread_systems: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
        ProcessConfig {
            runtime,
            threadpool,
        }: ProcessConfig<'_>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        if 1 < 0 {
            todo!("Give all systems a Mutex SystemStatus (if not already)");
            unreachable!()
        }

        let graph = Arc::new(RwLock::new(Graph::new(system_queue.clone(), links)));

        let thread_blacklist = Arc::new(RwLock::new(main_thread_systems));
        
        let program_details_map = Arc::new(program_details.into_iter().map(|program_details| {
            (program_details.get_system_program().clone().expect("Global Program shouldn't have an 'owner'"), program_details)
        }).collect::<HashMap<_, _>>());

        let (unwinder_tx, unwinder_rx) = std::sync::mpsc::channel();
        let (results_tx, results_rx) = std::sync::mpsc::channel();

        let threadpool = threadpool.and_then(|threadpool| Some((threadpool, threadpool.max_count())));

        if let Some((threadpool, thread_count)) = threadpool {
            for current_thread in 0..thread_count {
                let program_registry = Arc::clone(program_registry);

                let graph = Arc::clone(&graph);
                let program_details_map = Arc::clone(&program_details_map);
                let thread_blacklist = Arc::clone(&thread_blacklist);

                let results_tx = results_tx.clone();
                
                let thread_label = format!("Thread: {current_thread}");
                let unwinder = Unwinder::new(unwinder_tx.clone(), thread_label.clone());

                let runtime = runtime.cloned();
                threadpool.execute(move || { 
                    let results = Self::process_blocking_thread(
                        thread_label, 
                        &runtime, 
                        &graph, 
                        &program_registry, 
                        &thread_blacklist,
                        &program_details_map
                    );
                    
                    match results_tx.send(results.into_iter()) {
                        Ok(_) => {},
                        Err(_disconnected) => unreachable!(),
                    }

                    drop(unwinder);
                });
            }    
        }

        let main_thread_label = format!("Main Thread");

        // Then use the main thread to help finish executing the other systems
        let results = Self::process_blocking_thread(
            main_thread_label, 
            &runtime.cloned(), 
            &graph, 
            program_registry, 
            &Arc::new(RwLock::new(HashSet::default())),
            &program_details_map
        );

        match results_tx.send(results.into_iter()) {
            Ok(_) => {},
            Err(_disconnected) => unreachable!(),
        }

        
        if let Some((threadpool, thread_count)) = threadpool {
            for _ in 0..thread_count {
                let (panicked, thread_label) = unwinder_rx.recv().unwrap();

                // can try and "put_system"
                assert!(!panicked, "{}", format!("Thread Panicked: {thread_label}"));
            }

            threadpool.join();
        }

        drop(results_tx);

        results_rx.iter().flat_map(|results| results).collect()
    }

    fn process_blocking_thread(
        thread_label: String,
        runtime: &Option<Arc<Runtime>>,
        graph: &Arc<RwLock<Graph<SystemId>>>,
        program_registry: &Arc<ProgramRegistry>,
        blacklisted_systems: &Arc<RwLock<HashSet<SystemId>>>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        LABEL.with(|label| {
            label.replace(Some(thread_label));
        });

        if let Some(runtime) = runtime {
            runtime.block_on(async move {
                Self::execute_graph(graph, program_registry, blacklisted_systems, program_details_map)
            })
        } else {
            Self::execute_graph(graph, program_registry, blacklisted_systems, program_details_map)
        }
    }

    fn execute_graph(
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
                    );

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

            tasks.retain_mut(|(task, node)| {
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(result) => {
                        
                        let mut node = node.write();
                        let (program_id, system_entity) = node.data();

                        let program_details = program_details_map.get(program_id).cloned().unwrap_or_default();

                        let program_access_builder = program_details.into_access_builder();
                        
                        let world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder]);

                        if let Ok(Ok(world)) = world {
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
    
                                // false = do not retain- it is done
                                return false;
                            }
                        }



                        true
                    },
                    Poll::Pending => true,
                }
            });
        }
    
        results
    }

    fn run_node<'a>(
        node: &mut ArcRwLockWriteGuard<RawRwLock, Node<SystemId>>,
        program_registry: &Arc<ProgramRegistry>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>
    ) -> Option<ExecuteSystemResult<'a>> {
        let (program_id, system_entity) = node.data();

        let default_program_details = ProgramDetails::default();
        let program_details = program_details_map.get(program_id).or(Some(&default_program_details)).unwrap();

        match Self::run_system(program_registry, program_details, *system_entity) {
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

    fn run_system<'a>(
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        system_entity: Entity,
    ) -> Option<ExecuteSystemResult<'a>> {
        let program_access_builder = program_details.clone().into_access_builder();

        let world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder]);

        if let Ok(Ok(world)) = world {
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
                    );
        
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

    fn execute_system<'a>(
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
                ))
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

    fn execute_sync_system(
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

        let mut world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);

        // I know.. I know
            // i have given up 
        while !matches!(world, Ok(Ok(_))) {
            std::thread::yield_now();

            world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);
        }

        if let Ok(Ok(world)) = world {
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
        } else {
            unreachable!()
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

            let mut world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);

            // I know.. I know
            // i have given up 
            while !matches!(world, Ok(Ok(_))) {
                std::thread::yield_now();

                world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);
            }

            if let Ok(Ok(world)) = world {
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

    pub fn process_non_blocking(
        system_queue: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        runtime: &Arc<Runtime>,
        program_details: HashSet<ProgramDetails>,
    ) -> (
        Vec<JoinHandle<(SystemId, Result<Option<SystemResult>, SystemError>)>>, 
        Vec<tokio::task::JoinHandle<(SystemId, Result<Option<SystemResult>, SystemError>)>>
    ) {
        if 1 < 0 {
            todo!("Give all systems a Mutex SystemStatus (if not already)");
            unreachable!()
        }

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

            let mut world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);

            // I know.. I know
            // i have given up 
            while !matches!(world, Ok(Ok(_))) {
                std::thread::yield_now();

                world = program_registry.resolve::<Shared<World>>(None, vec![program_access_builder.clone()]);
            }

            if let Ok(Ok(world)) = world {
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
                                    let join_handle = std::thread::spawn(move || {
                                        let access_builders = if let Ok(Ok(world)) = program_registry
                                            .resolve::<Shared<World>>(None, vec![program_access_builder]) {
                                                let prepared_access_builders = world.prepare_get_shared::<&Vec<AccessBuilder>>(system_entity);
                                                let access_builders = prepared_access_builders.and_then(|access_builders| Some(access_builders.get(&world)));
                                                (*access_builders.as_deref().unwrap_or(&&vec![])).clone()
                                        } else { vec![] };

                                        let result = Self::execute_sync_system(
                                            sync_system, 
                                            system_entity, 
                                            &program_registry, 
                                            &program_details, 
                                            &access_builders
                                        );
                
                                        ((program_id, system_entity), result)
                                    });
                
                                    sync_handles.push(join_handle);
                                },
                                SystemKind::Async(async_system) => {
                                    let join_handle = runtime.spawn(async move {
                                        let access_builders = if let Ok(Ok(world)) = program_registry
                                            .resolve::<Shared<World>>(None, vec![program_access_builder]) {
                                                let prepared_access_builders = world.prepare_get_shared::<&Vec<AccessBuilder>>(system_entity);
                                                let access_builders = prepared_access_builders.and_then(|access_builders| Some(access_builders.get(&world)));
                                                (*access_builders.as_deref().unwrap_or(&&vec![])).clone()
                                        } else { vec![] };

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