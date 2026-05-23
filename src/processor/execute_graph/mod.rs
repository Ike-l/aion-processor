use std::{collections::{HashMap, HashSet}, sync::atomic::{AtomicBool, Ordering}, task::{Context, Poll, Waker}};

use aion_program::prelude::{ProgramId, ProgramRegistry};
use aion_system::prelude::{ProgramDetails, SystemResult};
use execution_graph::prelude::Graph;

use crate::prelude::{DumbWaker, ExecuteSystemResult, Processor, RunNode, SystemId, sync::{Arc, ArcRwLockWriteGuard, RwLock}};

pub struct ExecuteGraph;

impl Processor<ExecuteGraph> {
    pub async fn execute_graph(
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
        
        while !graph.read().await.is_finished() {
            let mut did_work = false;

            while let Some(leaf) = graph.read().await.find_leaves().pop() {
                if let Some(mut leaf) = leaf.try_write_arc() {
                    did_work = true;

                    assert!(leaf.is_ready());

                    if blacklisted_systems.read().await.contains(leaf.data()) {
                        continue
                    }

                    let result = Processor::<RunNode>::run_node(
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
                        Some(ExecuteSystemResult::Panicked(panic_error)) => {
                            results.insert(leaf.data().clone(), Some(SystemResult::Panicked(panic_error)));
                        }
                        // Will try again later
                        None => (),
                    }
                }
            }

            if !did_work  {
                std::thread::yield_now();
            }

            let mut continuing_tasks = Vec::new();
            for (mut task, node) in tasks.drain(..) {
                let mut done = false;
                match task.as_mut().poll(&mut context) {
                    Poll::Ready(result) => {     
                        let mut node = node.write();
                        let system_id = node.data().clone();
                        
                        match result {
                            Ok(Ok(result)) => {
                                results.insert(system_id, result);
                                
                                node.complete();
                            },
                            Ok(Err(_system_error)) => {
                                statuses.get(&system_id).expect("Statuses should contain ALL system ids (to be executed)").store(true, Ordering::SeqCst);
                            },
                            Err(panic_error) => {
                                results.insert(system_id, Some(SystemResult::Panicked(panic_error)));
    
                                node.complete();
                            },
                        }
                                
                        done = true;
                    },
                    Poll::Pending => {},
                }

                if !done {
                    continuing_tasks.push((task, node))
                }
            };

            tasks = continuing_tasks;

            if tasks.len() == 0 {
                std::thread::yield_now();
            }
        }
    
        results
    }
}