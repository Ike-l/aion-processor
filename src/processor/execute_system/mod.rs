use std::{any::Any, pin::Pin};

use aion_program::prelude::{AccessBuilder, ProgramRegistry};
use aion_system::{prelude::{ProgramDetails, SystemError, SystemResult}, system::SystemKind};
use hecs::Entity;

use crate::prelude::{ExecuteAsyncSystem, ExecuteSyncSystem, Processor, sync::Arc};

pub struct ExecuteSystem;

impl Processor<ExecuteSystem> {
    pub async fn execute_system<'a>(
        system_kind: SystemKind,
        program_registry: &Arc<ProgramRegistry>,
        program_details: &ProgramDetails,
        system_entity: Entity,
        access_builders: &Vec<AccessBuilder>
    ) -> Result<Result<Result<Option<SystemResult>, SystemError>, Box<dyn Any + Send>>, Pin<Box<dyn Future<Output = Result<Result<Option<SystemResult>, SystemError>, Box<dyn Any + Send>>> + Send + 'a>>> {
        match system_kind {
            SystemKind::Sync(sync_system) => {
                Ok(Processor::<ExecuteSyncSystem>::execute_sync_system(
                    sync_system,
                    system_entity,
                    program_registry,
                    program_details,
                    access_builders
                ).await)
            },
            SystemKind::Async(async_system) => {
                let task = Processor::<ExecuteAsyncSystem>::execute_async_system(
                    async_system,
                    system_entity,
                    Arc::clone(program_registry),
                    program_details.clone(),
                    access_builders.clone(),
                );

                Err(task)
            },
        }
    }
}