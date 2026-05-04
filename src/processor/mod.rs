use std::{cell::RefCell, collections::{HashMap, HashSet}, pin::Pin, task::{Context, Poll, Waker}};

use execution_graph::prelude::{Graph, Node};
use tokio::runtime::Runtime;

use crate::prelude::{GraphIdentifier, ProcessConfig, SystemQueue, Unique, sync::{RwLock, RwLockWriteGuard, Arc}, SystemCell, SystemStatus, DumbWaker};

use aion_program::prelude::{ProgramRegistry, PromptedProgramAccess, ProgramId, ResourceId};

use aion_system::prelude::{SystemResult, StoredSystem, StoredSystemMetadata, StoredSystemKind, SystemError};

// pub mod usage;
pub mod system_registry;
pub mod current_system_blockers;
pub mod process_config;
pub mod system_cell;
pub mod waker;

pub struct Processor;

thread_local! {
    static LABEL: RefCell<Option<String>> = RefCell::new(None);
}
// declare some must be run on the main thread?

// ProcessConfig 
// NonCollision- does not check accesses or reserve them

// SystemConfig
// MainThreadOnly
// HashMap<GraphIdentifier, (Vec<SystemConfig>, SystemEvent)>
impl Processor {
    pub fn process_blocking(
        graph: &Arc<RwLock<Graph<GraphIdentifier>>>,
        system_queue: SystemQueue,
        program_registry: &Arc<ProgramRegistry>,
        ProcessConfig {
            runtime,
            threadpool,
            collision_check
        }: ProcessConfig<'_>,
    ) -> HashMap<GraphIdentifier, SystemResult> {

        // collect systems from programs
        let system_identifiers = graph.read().nodes().iter().map(|node| {
            node.read().data().clone()
        }).collect::<HashSet<_>>();

        let systems = system_identifiers.iter().filter_map(|(program_id, system_id)| {
            let system_metadata = system_queue.get(&(&program_id, &system_id)).expect("`SystemQueue` holds all graphed `StoredSystemMetadata`");

           match program_registry.resolve::<Unique<StoredSystem>>(vec![PromptedProgramAccess {
                program_id: &program_id,
                program_password: system_metadata.program_password().as_ref(),
                user_details: system_metadata.user_details().as_ref().map(|(user_id, user_password)| { (user_id, user_password) }),
                resource_id: Some(system_id.clone()),
                resource_access: None,
                resource_password: None,
            }]) {
                Ok(Ok(mut stored_system)) => {
                    let system = stored_system.as_mut();

                    // Need these Cells specifically for Async functions because
                    // It could be multi-threaded 
                    // We need unique access to the system and
                    // If we were to use a guard to get the unique access,
                    // When saving the async task/future to poll later,
                    // It would need to be lifted out of the lifetime of the guard
                    // We can not preemptively store the systems at the same level of the container because threads steal work and storing them would effectively cancel that out
                    // So instead using a Cell we can get the unique access and store it
                    let system_cell = SystemCell::new(system.kind.take()?);
                    todo!("System Metadata Clone- needs to separate `criteria`");
                    Some(((program_id.clone(), system_id.clone()), (system_cell, (*system_metadata).clone())))
                },
                _ => None
            }
        });

        // Now any return MUST move the cells back into the systems

        let systems = Arc::new(systems.collect::<HashMap<_, _>>());
        
        if let Some(threadpool) = threadpool {
            for current_thread in 0..threadpool.max_count() {
                let graph = Arc::clone(graph);
                let program_registry = Arc::clone(program_registry);

                if let Some(runtime) = runtime {
                    let runtime = Arc::clone(runtime);
                    let systems = Arc::clone(&systems);

                    threadpool.execute(move || { 
                        // threadpool hides thread building so cannot set the name normally
                        LABEL.with(|label| {
                            label.replace(Some(format!("Thread: {current_thread}")));
                        });
                        
                        Self::async_execute(&runtime, &graph, &systems, &program_registry)
                    });
                } else {
                    Self::execute(&graph)
                }
            }    
        }

        if let Some(runtime) = runtime {
            Self::async_execute(runtime, graph, &systems, program_registry);
        } else {
            Self::execute(graph);
        }

        // catch any errors from threadpool

        // put cells back into systems

        // get results 
        
        todo!()
    }

    fn async_execute(
        runtime: &Runtime,
        graph: &RwLock<Graph<GraphIdentifier>>,
        systems: &HashMap<(ProgramId, ResourceId), (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>,
    ) {
        runtime.block_on(async move {
            let waker = Waker::from(Arc::new(DumbWaker));
            let mut context = Context::from_waker(&waker);
            let mut tasks: Vec<_> = Vec::new();
            
            while !graph.read().is_finished() {
                while let Some(leaf) = graph.read().find_leaves().pop() {
                    if let Some(mut leaf) = leaf.try_write() {
                        assert!(leaf.is_ready());

                        // Must only reference `system_cell`'s inner alongside its `status`
                        let result = unsafe { Self::run_system(
                            &mut leaf, 
                            systems,
                            program_registry,
                            &mut context,
                            &mut tasks
                        ) };

                        // send result
                    }
                }

                tasks.retain_mut(|(task)| {
                    match task.as_mut().poll(&mut context) {
                        Poll::Ready(result) => {
                            // send result
                            false
                        },
                        Poll::Pending => true,
                    }
                });
            }
        })
    }

    /// # Safety
    /// 
    /// `system_cell` should only be used in conjunction with `status`
    unsafe fn run_system<'a, 'b>(
        node: &mut RwLockWriteGuard<Node<GraphIdentifier>>,
        systems: &'a HashMap<(ProgramId, ResourceId), (SystemCell, StoredSystemMetadata)>,
        program_registry: &Arc<ProgramRegistry>,
        mut context: &mut Context,
        tasks: &'b mut Vec<Pin<Box<dyn Future<Output = Result<Option<SystemResult>, SystemError>> + Send + 'a>>>,
    ) -> Option<Result<Option<SystemResult>, SystemError>> {
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

                        // if !ReadOnly
                        // inner.reserve_accesses()
                        
                        *status = SystemStatus::Executing;

                        let result = match inner {
                            StoredSystemKind::Sync(stored_sync_system) => {
                                let result = stored_sync_system.execute(
                                    program_registry, 
                                    program_id, 
                                    stored_system_metadata.program_password().as_ref(), 
                                    stored_system_metadata.user_details().as_ref().map(|(user_id, user_password)| { (user_id, user_password) })
                                );

                                node.complete();

                                *status = SystemStatus::Executed;

                                Some(result)
                            },
                            StoredSystemKind::Async(stored_async_system) => {
                                let mut task = stored_async_system.execute(
                                    Arc::clone(program_registry),
                                    program_id.clone(),
                                    stored_system_metadata.program_password().clone(),
                                    stored_system_metadata.user_details().clone()
                                );

                                match task.as_mut().poll(&mut context) {
                                    Poll::Ready(result) => {
                                        node.complete();

                                        *status = SystemStatus::Executed;

                                        Some(result)
                                    },
                                    Poll::Pending => {
                                        node.make_pending();

                                        tasks.push((
                                            task
                                            // node
                                        ));

                                        None
                                    },
                                }
                            },
                        };

                        assert_ne!(*status, SystemStatus::Executing);

                        result
                    },
                    SystemStatus::Executing => unreachable!("function safety guarantees"),
                    SystemStatus::Pending |
                    SystemStatus::Executed => { None /* Is benign */ },
                }
            },
            None => unreachable!(),
        }
    }

    fn execute(
        graph: &RwLock<Graph<GraphIdentifier>>
    ) {
        
    }

    // pub fn process_non_blocking_finish
    // pub fn process_non_blocking_start
}