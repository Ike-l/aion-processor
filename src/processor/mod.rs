use std::{collections::{HashMap, HashSet}, sync::Arc};

use execution_graph::graph::Graph;
use threadpool::ThreadPool;
use tokio::runtime::Runtime;

use crate::prelude::{GraphIdentifier, ProcessConfig, SystemQueue, Unique};

use aion_program::prelude::{ProgramRegistry, PromptedProgramAccess, ResourceAccess};

use aion_system::prelude::{SystemResult, StoredSystem};

// pub mod usage;
pub mod system_registry;
pub mod current_system_blockers;
pub mod process_config;

pub struct Processor;

// declare some must be run on the main thread?

// ProcessConfig 
// NonCollision- does not check accesses or reserve them

// SystemConfig
// MainThreadOnly
// HashMap<GraphIdentifier, (Vec<SystemConfig>, SystemEvent)>
impl Processor {
    pub fn process_blocking(
        graph: Graph<GraphIdentifier>,
        system_queue: SystemQueue,
        program_registry: &Arc<ProgramRegistry>,
        ProcessConfig {
            runtime,
            threadpool,
            collision_check
        }: ProcessConfig<'_>,
    ) -> HashMap<GraphIdentifier, SystemResult> {

        // collect systems from programs
        let system_identifiers = graph.nodes().iter().map(|node| {
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
                    let system = stored_system.as_mut();//.kind.take()?;

                    let system_cell = system.into_cell()?;
                    Some(((program_id, system_id), (system_cell, system_metadata)))
                },
                _ => None
            }
        });

        let systems = Arc::new(systems.collect::<HashMap<_, _>>());
        

        
        let threads = if let Some(threadpool) = threadpool {
            threadpool.max_count()
        } else {
            0
        };

        if threads == 0 {
            // Self::execute_thread()
        } else {
            for current_thread in 0..threads {

            }
        }

        todo!()
    }

    // pub fn process_non_blocking_finish
    // pub fn process_non_blocking_start
}