pub mod processor;
pub mod parameters;

pub mod prelude {
    pub(crate) mod sync {
        pub use tokio::sync::RwLock;
        pub use parking_lot::ArcRwLockWriteGuard;
        pub use parking_lot::RawRwLock;
        pub use std::sync::Arc;
    }

    pub use crate::{
        parameters::{
            advanced::{
                disjoint::{
                    Disjoint
                },
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

    pub(crate) use crate::{
        processor::{
            execute_async_system::{
                ExecuteAsyncSystem
            },
            execute_graph::{
                ExecuteGraph
            },
            execute_sync_system::{
                ExecuteSyncSystem
            },
            execute_system::{
                ExecuteSystem
            },
            run_node::{
                RunNode
            },
            run_system::{
                RunSystem
            },
            test_systems::{
                TestSystems
            }
        }
    };

    use crate::processor::{blocking::Blocking, non_blocking::NonBlocking};
    pub type BlockingProcessor = Processor<Blocking>;
    pub type NonBlockingProcessor = Processor<NonBlocking>;

    use aion_program::prelude::ProgramId;
    pub type SystemId = (ProgramId, hecs::Entity);
}