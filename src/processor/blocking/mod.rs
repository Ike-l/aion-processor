use std::{cell::RefCell, collections::{HashMap, HashSet}, sync::atomic::AtomicBool};

use aion_program::prelude::ProgramRegistry;
use aion_system::prelude::{ProgramDetails, SystemResult};
use execution_graph::prelude::{Graph, Link};

use crate::prelude::{ExecuteGraph, Processor, SystemId, sync::{RwLock, Arc}};

thread_local! {
    static LABEL: RefCell<Option<String>> = RefCell::new(None);
}

pub struct Blocking;


// will keep trying to run all systems until they are done- so can be blocked if there is conflicting access
    // from a holder outside of the function

    // if there are systems which depend on a main thread system and there are no threads allocated
    // then will block infinitely

    // so go through the system queue and call check?
    /// REQUIRES A TOKIO RUNTIME WITH `.enter()`
    /// 
    /// To prevent indefinite blocking ensure the `World` resource is "public"
    /// 
    /// i.e take ownership of the `World` and then `whitelist` it
    /// 
    /// Best advice is to mock each system before putting it in `queue`
    /// 
    /// via Async/Sync System::check_accesses
    /// 
    /// This means that even if a system is blocked for the duration of this function (via a resolve which isn't dropped), 
    /// it will never be queued and so won't cause indefinite blocking

impl Processor<Blocking> {
    pub fn process_blocking(
        system_queue: HashSet<SystemId>,
        links: Vec<Link<SystemId>>,
        main_thread_systems: HashSet<SystemId>,
        program_registry: &Arc<ProgramRegistry>,
        program_details: HashSet<ProgramDetails>,
    ) -> HashMap<SystemId, Option<SystemResult>> {
        let statuses = Arc::new(system_queue.iter().map(|system_id| (system_id, AtomicBool::new(true))).collect::<HashMap<_, _>>());

        let graph = Arc::new(RwLock::new(Graph::new(system_queue.clone(), links)));

        let thread_blacklist = Arc::new(RwLock::new(main_thread_systems));
        
        let program_details_map = Arc::new(program_details.into_iter().map(|program_details| {
            (program_details.get_system_program().clone().expect("Global Program shouldn't have an 'owner'"), program_details)
        }).collect::<HashMap<_, _>>());

        let (results_tx, results_rx) = std::sync::mpsc::channel();

        let thread_count = 1;

        rayon::scope(|scope| {
            for current_thread in 0..thread_count {
                let program_registry = Arc::clone(program_registry);
        
                let graph = Arc::clone(&graph);
                let program_details_map = Arc::clone(&program_details_map);
                let thread_blacklist = Arc::clone(&thread_blacklist);
                let statuses = Arc::clone(&statuses);
        
                let results_tx = results_tx.clone();
                
                let runtime = tokio::runtime::Handle::current();

                scope.spawn(move |_| {
                    
                    let thread_label = format!("Thread: {current_thread}");
            
                    LABEL.with(|label| {
                        label.replace(Some(thread_label));
                    });
            
                    let results = runtime.block_on(
                        Processor::<ExecuteGraph>::execute_graph(&graph, &program_registry, &thread_blacklist, &program_details_map, &statuses)
                    );
            
                    let _ = results_tx.send(results);
                });
            }

            let main_thread_label = format!("Main Thread");

            LABEL.with(|label| {
                label.replace(Some(main_thread_label));
            });

            let main_results = tokio::runtime::Handle::current().block_on(
                Processor::<ExecuteGraph>::execute_graph(&graph, program_registry, &Arc::new(RwLock::new(HashSet::default())), &program_details_map, &statuses)
            );

            let _ = results_tx.send(main_results);
        });

        drop(results_tx);

        results_rx.iter().flat_map(|results| results).collect()
    }
}