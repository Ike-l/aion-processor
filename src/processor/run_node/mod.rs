use std::{collections::HashMap, sync::atomic::AtomicBool};

use aion_program::prelude::{ProgramId, ProgramRegistry};
use aion_system::prelude::ProgramDetails;
use execution_graph::prelude::Node;

use crate::prelude::{ExecuteSystemResult, Processor, RunSystem, SystemId, sync::{Arc, ArcRwLockWriteGuard, RawRwLock}};

pub struct RunNode;

impl Processor<RunNode> {
    pub async fn run_node<'a>(
        node: &mut ArcRwLockWriteGuard<RawRwLock, Node<SystemId>>,
        program_registry: &Arc<ProgramRegistry>,
        program_details_map: &HashMap<ProgramId, ProgramDetails>,
        statuses: &HashMap<&SystemId, AtomicBool>,
    ) -> Option<ExecuteSystemResult<'a>> {
        let (program_id, system_entity) = node.data();

        let default_program_details = ProgramDetails::default();
        let program_details = program_details_map.get(program_id).or(Some(&default_program_details)).unwrap();

        match Processor::<RunSystem>::run_system(program_registry, program_details, *system_entity, &(program_id.clone(), *system_entity), statuses).await {
            finished @ Some(ExecuteSystemResult::Final(_)) => {
                node.complete();

                finished
            },
            panicked @ Some(ExecuteSystemResult::Panicked(_)) => {
                node.complete();

                panicked
            }
            pending @ Some(ExecuteSystemResult::Pending(_)) => {
                node.make_pending();

                pending
            },
            None => None,
        }
    }
}