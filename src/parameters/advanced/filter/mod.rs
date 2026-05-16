use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError};
use hecs::Entity;

use crate::prelude::AccessBuilderFilter;

pub mod access_builder_filter;

pub struct Filter<'a, F: AccessBuilderFilter, T: Injection> {
    _f: PhantomData<F>,
    pub resources: HashMap<usize, T::Item<'a>>
}

impl<'a, F: AccessBuilderFilter, T: Injection> Injection for Filter<'a, F, T> {
    type Item<'new> = Filter<'new, F, T>;

    fn claim_manual_access_builders(accesses: Vec<&AccessBuilder>) -> Vec<usize> { T::claim_manual_access_builders(accesses) }

    fn submit_access(prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> {
        let mut finalised_accesses = Vec::new();

        let mut buffer = Vec::new();
        for prompted_access in prompted_accesses {
            match F::test_access_builder(&prompted_access) {
                true => {
                    buffer.push(prompted_access);
                    match T::submit_access(buffer.drain(..).collect()) {
                        Ok(instance_finalised_accesses) => {
                            finalised_accesses.extend(instance_finalised_accesses);
                        },
                        _ => (),
                    }
                },
                false => {
                    buffer.push(prompted_access)
                }
            }
        }

        Ok(finalised_accesses)
    }

    fn resolve_access<'new>(entity: Option<Entity>, program_registry: Arc<ProgramRegistry>, derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        let mut resources = HashMap::new();

        let mut buffer = Vec::new();
        for (i, derived_result) in derived_results.into_iter().enumerate() {
            match F::test_derived_result(&derived_result) {
                true => {
                    buffer.push(derived_result);
                    let instance_resolve_result = T::resolve_access(entity, Arc::clone(&program_registry), buffer.drain(..).collect());
                    if let Ok(resolved_access) = instance_resolve_result {
                        resources.insert(i, resolved_access);
                    }
                },
                false => {
                    buffer.push(derived_result);
                },
            }
        }

        Ok(Filter { resources, _f: PhantomData::default() })
    }
}