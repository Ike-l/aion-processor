// use aion_event::prelude::NextEvents;

// use crate::prelude::{ActiveExecutionGraph, Processor, ProgramMetadata, SystemId};

pub struct BlockingReadOnlyProcessor {}

// impl Processor for BlockingReadOnlyProcessor {
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