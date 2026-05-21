use std::{any::Any, pin::Pin};

use aion_system::prelude::{SystemResult, SystemError};

pub enum ExecuteSystemResult<'a> {
    Final(Option<SystemResult>), 
    Pending(Pin<Box<dyn Future<Output = Result<Option<SystemResult>, SystemError>> + Send + 'a>>),
    Panicked(Box<dyn Any + Send>)
}