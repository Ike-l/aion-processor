use std::{any::Any, panic::AssertUnwindSafe};

use aion_ecs::prelude::GetUnique;
use aion_program::prelude::{AccessBuilder, ProgramRegistry};
use aion_system::prelude::{ProgramDetails, SyncSystem, System, SystemError, SystemKind, SystemResult};
use hecs::Entity;

use crate::prelude::{Processor, sync::Arc};

pub struct ExecuteSyncSystem;

impl Processor<ExecuteSyncSystem>  {
    pub async fn execute_sync_system(
        mut sync_system: SyncSystem,
        system_entity: Entity,
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        access_builders: &Vec<AccessBuilder>
    ) -> Result<Result<Option<SystemResult>, SystemError>, Box<dyn Any + Send>> {
        let result = {
            std::panic::catch_unwind(AssertUnwindSafe(|| {
                sync_system.execute(
                    system_entity,
                    program_registry, 
                    program_details,                 
                    access_builders.iter().collect(),
                )
            }))  
        };

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
            system.get_unique().put_to_empty(SystemKind::Sync(sync_system))            
        }

        result
    }
}