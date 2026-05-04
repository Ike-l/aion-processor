pub mod processor;
pub mod parameters;

pub mod prelude {
    pub use crate::{
        parameters::{
            primitives::{
                unique::{
                    Unique
                }
            }
        },
        processor::{
            Processor,
            process_config::{
                ProcessConfig
            },
            // usage::{
            //     Usage,
            //     non_blocking_step::{
            //         NonBlockingStep
            //     }
            // },
            current_system_blockers::{
                CurrentSystemBlockers,
                system_blocker::{
                    SystemBlocker
                }
            },
            system_registry::{
                SystemRegistry,
                invariant::{
                    Invariant
                },
                system_queue::{
                    SystemQueue,
                }
            },
        }
    };

    use aion_program::prelude::{ProgramId, ResourceId};
    pub type GraphIdentifier = (ProgramId, ResourceId);
}