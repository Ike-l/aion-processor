use std::marker::PhantomData;

pub mod waker;
pub mod execute_system_result;

pub mod blocking;
pub mod execute_graph;
pub mod run_node;
pub mod run_system;
pub mod execute_system;
pub mod execute_sync_system;
pub mod execute_async_system;
pub mod non_blocking;


pub struct Processor<T> { _t: PhantomData<T> }
