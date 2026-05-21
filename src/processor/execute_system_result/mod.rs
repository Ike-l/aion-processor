use std::{any::Any, pin::Pin};

use aion_system::prelude::{SystemResult, SystemError};

pub enum ExecuteSystemResult<'a> {
    Final(Option<SystemResult>), 
    Pending(Pin<Box<dyn Future<Output = Result<Result<Option<SystemResult>, SystemError>, Box<dyn Any + Send>>> + Send + 'a>>),
    Panicked(Box<dyn Any + Send>)
}