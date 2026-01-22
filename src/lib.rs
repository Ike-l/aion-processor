pub mod system;
pub mod processor;

pub mod prelude {
    pub use crate::{
        processor::{
            Processor,
            program_metadata::ProgramMetadata,
            execution_graph::{
                ordering::Ordering,
                graph::{
                    ExecutionGraph,
                    node::Node,
                    node_map::NodeMap,
                    active_execution_graph::{
                        ActiveExecutionGraph,
                        execution_data::{
                            status::{
                                Status,
                                StatusKind
                            },
                            ExecutionData
                        }
                    }
                },
            },
        },
        system::{
            FunctionSystem,
            system_result::{
                SystemResult,
                system_result_builder::SystemResultBuilder
            },
            system_metadata::{
                system_id::SystemId,
                criteria::{
                    Criteria
                },
                processor_ordering::{
                    ProcessorOrdering,
                }
            },
            stored_system::{
                StoredSystem,
                system_access_descriptor::SystemAccessDescriptor,
                system_resource_descriptor::SystemResourceDescriptor
            },
        }
    };
}