use std::collections::{HashMap, HashSet};

use aion_ecs::prelude::{GetOwned, GetUnique};
use aion_program::prelude::{AccessBuilder, ProgramId, ProgramRegistry};
use aion_system::prelude::{ProgramDetails, System, SystemKind};

use crate::prelude::{Processor, SystemId, sync::Arc};

pub struct TestSystems;

impl Processor<TestSystems> {
    pub async fn test_systems(
        queue: &HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
    ) -> HashSet<SystemId> {
        let mut passing_systems = HashSet::default();
        for (program_id, system_entity) in queue {
            let program_details = program_details_map.get(program_id).cloned().unwrap_or(ProgramDetails {
                system_program: Some(program_id.clone()),
                ..Default::default() 
            });

            let program_access_builder = program_details.clone().into_access_builder();

            let system = program_registry.resolve_async::<GetUnique<System>>(Some(*system_entity), vec![program_access_builder.clone()]);
            let system = match system {
                Ok(Ok(system)) => system,
                Ok(Err(future_system)) => future_system.await,
                Err(_) => continue,
            };

            let access_builders = program_registry.resolve_async::<GetOwned<Vec<AccessBuilder>, Vec<AccessBuilder>>>(Some(*system_entity), vec![program_access_builder]);
                    
            let access_builders = match access_builders {
                Ok(Ok(access_builders)) => access_builders.item,
                Ok(Err(future_access_builders)) => future_access_builders.await.item,
                Err(_) => vec![]
            };

            let result = match system.get_unique().take_system() {
                Some(SystemKind::Sync(sync_system)) => {
                    sync_system.check_accesses(*system_entity, program_registry, &program_details, access_builders.iter().collect())
                },
                Some(SystemKind::Async(async_system)) => {
                    async_system.check_accesses(*system_entity, program_registry, &program_details, access_builders.iter().collect())
                },
                None => continue,
            };

            if result {
                passing_systems.insert((program_id.clone(), *system_entity));
            }
        }

        passing_systems
    }
}