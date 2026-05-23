use std::{any::Any, panic::AssertUnwindSafe, pin::Pin};

use aion_ecs::prelude::GetUnique;
use aion_program::prelude::{AccessBuilder, ProgramRegistry};
use aion_system::{prelude::{AsyncSystem, ProgramDetails, System, SystemError, SystemResult}, system::SystemKind};
use futures::prelude::future::FutureExt;
use hecs::Entity;

use crate::prelude::{Processor, sync::Arc};

pub struct ExecuteAsyncSystem;

impl Processor<ExecuteAsyncSystem> {
    pub fn execute_async_system(
        mut async_system: AsyncSystem,
        system_entity: Entity,
        program_registry: Arc<ProgramRegistry>,
        program_details: ProgramDetails,
        access_builders: Vec<AccessBuilder>,
    ) -> Pin<Box<impl Future<Output = Result<Result<Option<SystemResult>, SystemError>, Box<dyn Any + Send>>>>> {       
       Box::pin(async move {   
            let future = async_system.execute(
                 system_entity,
                 Arc::clone(&program_registry),
                 program_details.clone(),
                 access_builders,
            );
     
            let wrapped_future = AssertUnwindSafe(future).catch_unwind();

            let result = wrapped_future.await;

            let program_access_builder = program_details.clone().into_access_builder();

            let prepared_system = program_registry.resolve_async::<GetUnique<System>>(Some(system_entity), vec![program_access_builder]);

            let system = match prepared_system {
                Ok(Ok(system)) => Some(system),
                Ok(Err(future_system)) => Some(future_system.await),
                // means the "submission" either 
                // 1. didn't have the correct input- should be unreachable! since we give the `program_access_builder`
                // 2. the resource couldn't be resolved because there wasn't enough inputs- unreachable! if 1. is unreachable!
                Err(_) => unreachable!(),
            };

            if let Some(system) = system {
                system.get_unique().put_to_empty(SystemKind::Async(async_system))            
            } 

            result
        })
    }
}