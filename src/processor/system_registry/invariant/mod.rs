use std::collections::HashMap;

use crate::prelude::{GraphIdentifier, SystemCriteria};

pub mod system_criteria;

pub enum Invariant<'a> {
    EventCondition(&'a HashMap<GraphIdentifier, SystemCriteria>)
}