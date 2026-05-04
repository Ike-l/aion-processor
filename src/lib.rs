pub mod processor;

pub mod prelude {
    pub use crate::{
        processor::{
            Processor,
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
}