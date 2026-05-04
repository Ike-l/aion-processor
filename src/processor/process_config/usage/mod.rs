pub mod non_blocking_step;

use crate::prelude::NonBlockingStep;

pub enum Usage {
    Blocking,
    NonBlocking(NonBlockingStep)
}