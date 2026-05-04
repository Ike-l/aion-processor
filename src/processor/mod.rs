pub mod process_config;

use crate::prelude::{ProcessConfig, Usage, NonBlockingStep};

pub struct Processor;

impl Processor {
    pub fn process(
        ProcessConfig {
            iter_method,
            invariants,
            usage
        }: &ProcessConfig,
    ) {
        match usage {
            Usage::Blocking => todo!(),
            Usage::NonBlocking(NonBlockingStep::Finish) => todo!(),
            Usage::NonBlocking(NonBlockingStep::Start) => todo!(),
        }
    }
}