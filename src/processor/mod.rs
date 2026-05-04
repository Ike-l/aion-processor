use std::sync::Arc;

use crate::prelude::{ProcessConfig, Usage, NonBlockingStep, SystemRegistry, Invariant, CurrentSystemBlockers};

use aion_event::prelude::CurrentEvents;
use aion_program::prelude::{ProgramRegistry};
use aion_system::prelude::StoredSystemMetadata;

pub mod process_config;
pub mod system_registry;
pub mod current_system_blockers;

pub struct Processor;

impl Processor {
    // preprocess: 
    // given system registry
    // filter systems by blockers
    // filter systems by events
    // return Vec<SystemMetadata
    // pub fn process(
    //     ProcessConfig {
    //         iter_method,
    //         invariants,
    //         usage
    //     }: &ProcessConfig,
    //     program_registry: &Arc<ProgramRegistry>,
    //     system_registry: &SystemRegistry,
    //     blockers: &SystemBlockers,
    //     events: &SystemEvents
    // ) {
    //     let queue = Self::enqueue(invariants, program_registry, system_registry);
    //     // get systems from memory:

    //     let 

    //     // divide systems into forest for greedy/chunk for sequential/chunks for chunked

    //     // match usage {
    //     //     Usage::Blocking => todo!(),
    //     //     Usage::NonBlocking(NonBlockingStep::Finish) => todo!(),
    //     //     Usage::NonBlocking(NonBlockingStep::Start) => todo!(),
    //     // }
    // }
}