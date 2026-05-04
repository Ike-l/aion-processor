pub mod processor;
pub mod parameters;

pub mod prelude {
    pub(crate) mod sync {
        pub use parking_lot::Mutex;
        pub use parking_lot::RwLock;
        pub use parking_lot::RwLockWriteGuard;
        pub use std::sync::Arc;
    }

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
            system_cell::{
                SystemCell,
                system_status::{
                    SystemStatus
                }
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