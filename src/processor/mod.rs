use std::{cell::RefCell, collections::HashMap, pin::Pin, task::{Context, Poll, Waker}};

use execution_graph::prelude::{Graph, Node};
use tokio::runtime::Runtime;

use crate::prelude::{GraphIdentifier, ProcessConfig, SystemQueue, Unique, sync::{RwLock, ArcRwLockWriteGuard, RawRwLock, Arc}, SystemCell, SystemStatus, DumbWaker, Unwinder};

use aion_program::prelude::{AccessBuilder, ProgramRegistry, ProgramId, ResourceId};

use aion_system::prelude::{SystemResult, StoredSystem, StoredSystemMetadata, StoredSystemKind, SystemError};

pub mod system_registry;
pub mod current_system_blockers;
pub mod process_config;
pub mod system_cell;
pub mod waker;
pub mod unwinder;

thread_local! {
    static LABEL: RefCell<Option<String>> = RefCell::new(None);
}

pub struct Processor;

impl Processor {
    // will keep trying to run all systems until they are done- so can be blocked if there is conflicting access
    // from a holder outside of the function
    pub fn process_blocking(
        graph: &Arc<RwLock<Graph<GraphIdentifier>>>,
        main_thread_graph: &Arc<RwLock<Graph<GraphIdentifier>>>,
        system_queue: SystemQueue,
        program_registry: &Arc<ProgramRegistry>,
        ProcessConfig {
            runtime,
            threadpool,
        }: ProcessConfig<'_>,
    ) -> HashMap<(ProgramId, ResourceId), Option<SystemResult>> {
        // If a `system` is in both `main_thread_graph` & `graph`
        // THIS ordering ensures it will prioritise the main thread 
        let main_thread_systems = Self::get_systems(&system_queue, program_registry);
        let systems = Self::get_systems(&system_queue, program_registry);

        // Now any return MUST move the cells back into the systems

        let systems = Arc::new(systems);
        
        let (unwinder_tx, unwinder_rx) = std::sync::mpsc::channel();
        let (results_tx, results_rx) = std::sync::mpsc::channel();

        let threadpool = threadpool.and_then(|threadpool| Some((threadpool, threadpool.max_count())));

        if let Some((threadpool, thread_count)) = threadpool {
            for current_thread in 0..thread_count {
                let program_registry = Arc::clone(program_registry);

                let graph = Arc::clone(graph);
                let systems = Arc::clone(&systems);

                let results_tx = results_tx.clone();
                
                let thread_label = format!("Thread: {current_thread}");
                let unwinder = Unwinder::new(unwinder_tx.clone(), thread_label.clone());

                let runtime = Arc::clone(&runtime);
                threadpool.execute(move || { 
                    let results = Self::process_blocking_thread(thread_label, runtime, &graph, &systems, &program_registry);
                    
                    match results_tx.send(results.into_iter()) {
                        Ok(_) => {},
                        Err(_disconnected) => unreachable!(),
                    }

                    drop(unwinder);
                });
            }    
        }

        let main_thread_label = format!("Main Thread");

        // Execute all systems for the main thread
        let main_thread_runtime = Arc::clone(&runtime);
        let results = Self::process_blocking_thread(main_thread_label.clone(), main_thread_runtime, main_thread_graph, &main_thread_systems, program_registry);

        match results_tx.send(results.into_iter()) {
            Ok(_) => {},
            Err(_disconnected) => unreachable!(),
        }

        // Then use the main thread to help finish executing the other systems
        let results = Self::process_blocking_thread(main_thread_label, runtime, graph, &systems, program_registry);

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

        Self::put_systems(Arc::try_unwrap(systems).unwrap(), program_registry);
        Self::put_systems(main_thread_systems, program_registry);
        
        drop(results_tx);
        results_rx.iter().flat_map(|m| m).collect()
    }

    fn put_systems(
        systems: HashMap<GraphIdentifier, (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>
    ) {
        for ((program_id, system_resource_id), (system_cell, system_metadata)) in systems {
            let prompted_access = AccessBuilder {
                program_id: Some(&program_id),
                program_password: system_metadata.system_program_password().as_ref(),
                user_details: system_metadata.user_details().as_ref().map(|(user_id, user_password)| { (user_id, user_password) }),
                resource_id: Some(system_resource_id.clone()),
                resource_access: None,
                resource_password: system_metadata.system_resource_password().as_ref(),
            };

           match program_registry.resolve::<Unique<StoredSystem>>(vec![prompted_access]) {
                Ok(Ok(mut stored_system)) => {
                    let system = stored_system.as_mut();

                    // # Safety
                    // We always use `status` when referencing `system`
                    // `SystemCell` is owned here
                    let status = system_cell.status.lock();
                    assert!(*status != SystemStatus::Executing || *status != SystemStatus::Pending);

                    system.kind.replace(unsafe { system_cell.take() });
                },
                // system will now vanish into the aether
                _ => ()
            }
        }
    }

    fn get_systems(
        system_queue: &SystemQueue,
        program_registry: &Arc<ProgramRegistry>
    ) -> HashMap<GraphIdentifier, (SystemCell, StoredSystemMetadata)> {
        system_queue.systems().into_iter().filter_map(|((program_id, system_resource_id), system_metadata)| {
            let prompted_access = AccessBuilder {
                program_id: Some(&program_id),
                program_password: system_metadata.system_program_password().as_ref(),
                user_details: system_metadata.user_details().as_ref().map(|(user_id, user_password)| { (user_id, user_password) }),
                resource_id: Some((*system_resource_id).clone()),
                resource_access: None,
                resource_password: None,
            };

           match program_registry.resolve::<Unique<StoredSystem>>(vec![prompted_access]) {
                Ok(Ok(mut stored_system)) => {
                    let system = stored_system.as_mut();
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

                    let system_cell = SystemCell::new(system.kind.take()?);
                    Some((((*program_id).clone(), (*system_resource_id).clone()), (system_cell, (*system_metadata).clone())))
                },
                _ => None
            }
        }).collect()
    }

    fn process_blocking_thread(
        thread_label: String,
        runtime: Arc<Option<Runtime>>,
        graph: &Arc<RwLock<Graph<GraphIdentifier>>>,
        systems: &HashMap<GraphIdentifier, (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>,
    ) -> HashMap<GraphIdentifier, Option<SystemResult>> {
        LABEL.with(|label| {
            label.replace(Some(thread_label));
        });

        if let Some(runtime) = (*runtime).as_ref() {
            runtime.block_on(async move {
                Self::execute_graph(graph, &systems, program_registry)
            })
        } else {
            Self::execute_graph(graph, &systems, program_registry)
        }
    }

    fn execute_graph(
        graph: &RwLock<Graph<GraphIdentifier>>,
        systems: &HashMap<(ProgramId, ResourceId), (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>,
    ) -> HashMap<GraphIdentifier, Option<SystemResult>> {
        let mut results = HashMap::new();

        let waker = Waker::from(Arc::new(DumbWaker));
        let mut context = Context::from_waker(&waker);
        let mut tasks: Vec<_> = Vec::new();
        
        while !graph.read().is_finished() {
            while let Some(leaf) = graph.read().find_leaves().pop() {
                if let Some(mut leaf) = leaf.try_write_arc() {
                    assert!(leaf.is_ready());

                    // Must only reference `system_cell`'s inner alongside its `status`
                    let result = unsafe { Self::run_system(
                        &mut leaf, 
                        systems,
                        program_registry,
                    ) };

                    match result {
                        Some(result) => {
                            match result {
                                Ok(result) => {
                                    results.insert(leaf.data().clone(), result);
                                },
                                Err(task) => {
                                    tasks.push((task, ArcRwLockWriteGuard::into_arc(leaf)));
                                },
                            }
                        },
                        None => todo!(),
                    }
                }
            }

            tasks.retain_mut(|(task, node)| {
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(result) => {
                        
                        let mut node = node.write();
                        let identifier = node.data();

                        let Some((system_cell, _)) = systems.get(identifier) else { panic!("Expected `systems` to contains all `graph` nodes") };

                        let mut status = system_cell.status.lock();

                        match result {
                            Ok(result) => {
                                results.insert(identifier.clone(), result);
                                *status = SystemStatus::Executed;
                                node.complete();
                            },
                            Err(_system_error) => {
                                *status = SystemStatus::Ready;                                
                            },
                        }

                        false
                    },
                    Poll::Pending => true,
                }
            });
        }
    
        results
    }

    /// # Safety
    /// 
    /// `system_cell` should only be used in conjunction with `status`
    unsafe fn run_system<'a>(
        node: &mut ArcRwLockWriteGuard<RawRwLock, Node<GraphIdentifier>>,
        systems: &'a HashMap<(ProgramId, ResourceId), (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>,
    ) -> Option<Result<Option<SystemResult>, Pin<Box<dyn Future<Output = Result<Option<SystemResult>, SystemError>> + Send + 'a>>>> {
        let identifier = node.data();
        let Some((system_cell, stored_system_metadata)) = systems.get(identifier) else { panic!("Expected `systems` to contains all `graph` nodes") };

        let program_id = &identifier.0;

        match system_cell.status.try_lock() {
            Some(mut status) => {
                match *status {
                    SystemStatus::Ready => {
                        // Safety
                        //
                        // We use the `status`
                        let inner = unsafe {
                            system_cell.get()
                        };

                        *status = SystemStatus::Executing;
                        let result = Self::execute_system(
                            inner,
                            program_registry,
                            stored_system_metadata.get_builders(program_id)
                        ); 

                        match result {
                            Ok(Ok(result)) => {
                                // think here 
                                node.complete();
        
                                *status = SystemStatus::Executed;

                                Some(Ok(result))
                            },
                            Ok(Err(_system_error)) => {
                                *status = SystemStatus::Ready;

                                None
                            }
                            Err(task) => {
                                node.make_pending();
        
                                *status = SystemStatus::Pending;

                                Some(Err(task))
                            },
                        }
                    },
                    SystemStatus::Executing => unreachable!("function safety guarantees"),
                    SystemStatus::Pending |
                    SystemStatus::Executed => { None /* Is benign */ },
                }
            },
            None => unreachable!(),
        }
    }

    fn execute_system<'a, 'b>(
        inner: &'a mut StoredSystemKind,
        program_registry: &Arc<ProgramRegistry>,
        (
            auto_access_builder,
            manual_access_builders
        ): (
            AccessBuilder<'b>, 
            Vec<AccessBuilder<'b>>
        )
    ) -> Result<Result<Option<SystemResult>, SystemError>, Pin<Box<dyn Future<Output = Result<Option<SystemResult>, SystemError>> + Send + 'a>>> {
        match inner {
            StoredSystemKind::Sync(stored_sync_system) => {
                let result = stored_sync_system.execute(
                    program_registry, 
                    &auto_access_builder,
                    manual_access_builders.iter().collect(),
                );

                Ok(result)
            },
            StoredSystemKind::Async(stored_async_system) => {
                let task = stored_async_system.execute(
                    Arc::clone(program_registry),
                    auto_access_builder.into(),
                    manual_access_builders.into_iter().map(|access_builder| access_builder.into()).collect(),
                );

                Err(task)
                // match task.as_mut().poll(&mut context) {
                //     Poll::Ready(result) => {
                //         Ok(result)
                //     },
                //     Poll::Pending => {
                //         Err(task)
                //     },
                // }
            },
        }
    }

    // todo:
    // make into multi layered
    // possibly use execute system
    pub fn process_non_blocking(
        system_queue: SystemQueue,
        program_registry: &Arc<ProgramRegistry>,
        runtime: &Arc<Runtime>
    ) {
        let systems = Self::get_systems(&system_queue, program_registry);

        let mut sync_handles = Vec::new();
        let mut async_handles = Vec::new();

        for ((program_id, system_resource_id), (system_cell, stored_system_metadata)) in systems.into_iter() {
            let mut status = system_cell.status.lock();
            *status = SystemStatus::Executing;
            // Safety
            // Uses `status`
            // system cell is also owned here
            let system = unsafe { system_cell.take() };

            let program_registry = Arc::clone(program_registry);
            let thread_program_id = program_id.clone();
            match system {
                StoredSystemKind::Sync(mut sync_system) => {
                    let join_handle = std::thread::spawn(move || {
                        let (auto_access_builder, manual_access_builders) = stored_system_metadata.get_builders(&thread_program_id);

                        let result = sync_system.execute(
                            &program_registry, 
                            &auto_access_builder, 
                            manual_access_builders.iter().collect()
                        );

                        (StoredSystemKind::Sync(sync_system), result)
                    });

                    sync_handles.push(((program_id, system_resource_id), join_handle));
                },
                StoredSystemKind::Async(mut async_system) => {
                    let join_handle = runtime.spawn(async move {

                        let user_details = stored_system_metadata.user_details().as_ref().map(|(user_id, user_password)| { (user_id, user_password) });
                        let auto_access_builder = AccessBuilder {
                            program_id: Some(&thread_program_id),
                            program_password: stored_system_metadata.system_program_password().as_ref(),
                            user_details,

                            resource_id: None,
                            resource_access: None,
                            resource_password: None,
                        };

                        let stored_access_builders = stored_system_metadata.stored_access_builders();
                        let manual_access_builders: Vec<_> = stored_access_builders.iter().map(|stored_access_builder| {
                        AccessBuilder {
                            program_id: stored_access_builder.program_id.as_ref(),
                            program_password: stored_access_builder.program_password.as_ref(),
                            user_details,
                            resource_id: stored_access_builder.resource_id.clone(),
                            resource_access: stored_access_builder.resource_access.clone(),
                            resource_password: stored_access_builder.resource_password.as_ref(),
                        }
                    }).collect();

                        let result = async_system.execute(
                            program_registry, 
                            auto_access_builder.into(), 
                            manual_access_builders.into_iter().map(|access_builder| access_builder.into()).collect(),
                        ).await;

                        (StoredSystemKind::Async(async_system), result)
                    });

                    async_handles.push(((program_id, system_resource_id), join_handle));
                },
            }
        }
    }
}