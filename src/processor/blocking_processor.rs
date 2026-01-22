// use aion_event::prelude::NextEvents;

// use crate::prelude::{ActiveExecutionGraph, Processor, ProgramMetadata, SystemId};

use crate::prelude::{ActiveExecutionGraph, ExecutionGraph, Processor};

pub struct BlockingProcessor {}

impl BlockingProcessor {
    pub fn run() {
        let foobar = Processor::filter_program_systems(
            program_registry, 
            programs, 
            systems, 
            current_events
        );

        // for each program/system id spawn event into next event

        let mut all_groups = Vec::new();
        for (program_id, systems) in foobar {
            let groups = Processor::group_dependent();
            all_groups.extend(groups);
        }

        let mut execution_graphs = Vec::new();
        for groups in all_groups.chunks(chunk_size) {
            let flat_group = groups.flat();
            execution_graphs.push(ExecutionGraph::new(flat_group));
        }

        active_execution_graphs: Vec<ActiveExecutionGraph<(SystemId, ProgramId)>> = execution_graphs.into_iter().map(|a| a.into()).collect();

        let results = Self::execute();

        Processor::parse_results(results, &mut next_events);
    }

    pub fn execute() {

    }
}

// impl Processor for BlockingProcessor {
//     fn execute(
//             &self,
//             program_registry: std::sync::Arc<aion_program::prelude::ProgramRegistry>,
//             // Arc ? 
//             // arc because to start an async task?
//             active_execution_graphs: Vec<ActiveExecutionGraph<SystemId>>,
//             system_registry: std::collections::HashMap<SystemId, ProgramMetadata>,
//             threadpool: threadpool::ThreadPool,
//             runtime: tokio::runtime::Runtime,
//         ) -> NextEvents {
//         todo!()
//     }
// }