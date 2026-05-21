pub mod processor;
pub mod parameters;

pub mod prelude {
    pub(crate) mod sync {
        #[allow(unused)]
        pub use parking_lot::Mutex;
        
        pub use parking_lot::RwLock;
        pub use parking_lot::ArcRwLockWriteGuard;
        pub use parking_lot::RawRwLock;
        pub use std::sync::Arc;
    }

    pub use crate::{
        parameters::{
            advanced::{
                get_program_registry::{
                    GetProgramRegistry
                },
                filter::{
                    Filter,
                    access_builder_filter::{
                        AccessBuilderFilter,
                        user_details_filter::{
                            UserDetailsGetter,
                            UserDetailsFilter,
                        },
                        resource_id_filter::{
                            ResourceIdGetter,
                            ResourceIdFilter,
                        }
                    }
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
        }
    };

    use aion_program::prelude::ProgramId;
    pub type SystemId = (ProgramId, hecs::Entity);
}