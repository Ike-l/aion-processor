use std::{collections::{HashMap, HashSet}, sync::Arc};

use aion_event::prelude::{CurrentEvents, NextEvents};
use aion_program::prelude::{ProgramId, ProgramRegistry};
use tokio::runtime::Runtime;
use threadpool::ThreadPool;

use crate::{prelude::{ActiveExecutionGraph, ProgramMetadata, StoredSystem, SystemId}, system::system_metadata::SystemMetadata};

pub mod program_metadata;

pub mod blocking_processor;
pub mod non_blocking_processor;
pub mod read_only_processor;


pub struct Processor { }
impl Processor {
    fn filter_program_systems<'a>(
        // will put injection primitives in Processor
        // so can cut down on parameters to only Vec<ProgramMetadata> ?
        program_registry: &ProgramRegistry,
        programs: &HashMap<ProgramId, ProgramMetadata>,
        systems: HashMap<ProgramId, HashMap<&'a SystemId, (&SystemMetadata, &StoredSystem)>>,
        current_events: &CurrentEvents,
        // blockers?
    ) -> HashMap<ProgramId, HashSet<&'a SystemId>> {
        let mut filtered_program_systems = HashMap::new();
        let default_program_metadata = ProgramMetadata::default();

        for (program_id, systems) in systems {
            let program_metadata = programs.get(&program_id).unwrap_or(&default_program_metadata);

            let filtered_systems = systems
                .into_iter()
                // .filter(|(system_id, _)| !current_blockers.is_blocking(system_id))
                .filter(|(_, (system_metadata, _))| {
                    system_metadata.test(current_events)
                })
                // check for resources
                // note: Resources could be removed, would either cause panic or indefinite blocking
                .filter(|(_, (_, stored_system))| {
                    let resource_ids = stored_system.get_resource_descriptor();

                    program_registry.contains_resources(
                        Some(&program_id),
                        program_metadata.program_key_id().as_ref(),
                        None,
                        resource_ids.get_resources()
                    ).unwrap_or(false)
                })
                // check for accesses
                // note: accesses could change, 
                // if incompatible accesses held over execute would cause indefinite blocking
                .filter(|(_, (_, stored_system))| {
                    let accesses = stored_system.get_access_descriptor();

                    let accesses = accesses.get_accesses();
                    let accesses = accesses
                        .into_iter()
                        .map(|(
                            resource_id, 
                            resource_access, 
                            resource_reserver_id,
                            resource_key_id
                        )| {
                            (resource_id, resource_access, resource_reserver_id.as_ref(), resource_key_id.as_ref())
                        }).collect::<Vec<_>>();

                    program_registry.permits_accesses(
                        Some(&program_id), 
                        program_metadata.program_key_id().as_ref(), 
                        None, 
                        &accesses
                    ).unwrap_or(false)
                })
                .map(|(system_id, _)| system_id)
                .collect();

            filtered_program_systems.insert(program_id, filtered_systems);
        }

        filtered_program_systems
    }

    // called for each program, join into one large array of dependent
    // since can pre-divide by program since know they can't be dependent
    // since removed ability to access "global" memory (add back?)
    fn group_dependent() {}

    // filter -> divide by program -> divide by borrows -> execute

    // large array of dependent into large array of execution graphs
    // into large array of active execution graphs


    // fn execute(
    //     &self,
    //     program_registry: Arc<ProgramRegistry>,
    //     // Arc ? 
    //     // arc because to start an async task?
    //     active_execution_graphs: Vec<ActiveExecutionGraph<(SystemId, ProgramId)>>,
    //     system_registry: HashMap<SystemId, ProgramMetadata>,
    //     threadpool: ThreadPool,
    //     runtime: Runtime,
    // ) -> NextEvents;

}


impl EventSystem for Processor {
    fn execute() {
        // end background

        // update blockers?

        // execute_blocking
        // execute_read_only
        // execute_non_blocking
    }
}