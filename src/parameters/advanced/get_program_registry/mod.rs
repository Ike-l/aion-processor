use std::sync::Arc;

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError};

pub struct GetProgramRegistry {
    pub program_registry: Arc<ProgramRegistry>
}

impl Injection for GetProgramRegistry {
    type Item<'new> = GetProgramRegistry;
 
    fn claim_manual_access_builders(_accesses: Vec<&AccessBuilder>) -> Vec<usize> { vec![] }

    fn submit_access(_prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> {
        Ok(vec![])
    }

    fn resolve_access<'new>(program_registry: Arc<ProgramRegistry>, _derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        Ok(GetProgramRegistry { program_registry })
    }
}