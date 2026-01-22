use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use aion_program::prelude::{Program, ResourceKeyId};

pub mod system_metadata;
pub mod stored_system;
pub mod system_result;

pub struct FunctionSystem<Input, F> {
    f: F,
    _layout: PhantomData<fn() -> Input>
}

// A "System" is simply a collection of Injection parameters
pub trait System {
    fn execute(
        &mut self, 
        program: Arc<Program>,
        program_keys: HashMap<usize, ResourceKeyId> 
    ); 
}