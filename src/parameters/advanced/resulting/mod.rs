use std::sync::Arc;

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError};

pub struct Resulting<'a, T: Injection> {
    pub result: Result<T::Item<'a>, ResolveResourceError>
}

impl<'a, T: Injection> Injection for Resulting<'a, T> {
    type Item<'new> = Resulting<'new, T>;

    fn claim_manual_access_builders(accesses: Vec<&AccessBuilder>) -> Vec<usize> { T::claim_manual_access_builders(accesses) }
    fn submit_access(prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> { T::submit_access(prompted_accesses) }

    fn resolve_access<'new>(program_registry: Arc<ProgramRegistry>, derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        Ok(Resulting {
            result: T::resolve_access(program_registry, derived_results)
        })
    }
}