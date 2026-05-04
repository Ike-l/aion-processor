pub mod iter_method;
pub mod invariant;
pub mod usage;

use crate::prelude::{Invariant, IterMethod, Usage};

pub struct ProcessConfig {
    pub iter_method: IterMethod,
    pub invariants: Vec<Invariant>,
    pub usage: Usage
}