pub mod processor;
pub mod parameters;

pub mod prelude {
    pub(crate) mod sync {
        pub use parking_lot::Mutex;
        pub use parking_lot::RwLock;
        pub use parking_lot::ArcRwLockWriteGuard;
        pub use parking_lot::RawRwLock;
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
            waker::{
                DumbWaker
            },
            execute_system_result::{
                ExecuteSystemResult
            },
            unwinder::{
                Unwinder
            },
            process_config::{
                ProcessConfig
            },
            system_cell::{
                SystemCell,
                system_status::{
                    SystemStatus
                }
            },
            // invariant::{
            //     Invariant,
            //     system_criteria::{
            //         SystemCriteria
            //     }
            // },
            system_queue::{
                SystemQueue,
            }
        }
    };

    use aion_program::prelude::{ProgramId, ResourceId};
    pub type GraphIdentifier = (ProgramId, ResourceId);
}