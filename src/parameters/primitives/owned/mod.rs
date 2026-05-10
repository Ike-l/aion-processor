use std::{marker::PhantomData, sync::Arc};

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError};

use crate::prelude::Shared;

pub struct Owned<F, O> {
    pub resource: O,
    _f: PhantomData<F>
}

impl<F: 'static, O> Injection for Owned<F, O> 
    where F: ToOwned<Owned = O>
{
    type Item<'new> = Owned<F, O>;

    fn claim_manual_access_builders(accesses: Vec<&AccessBuilder>) -> Vec<usize> { Shared::<F>::claim_manual_access_builders(accesses) }

    fn submit_access(prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> { Shared::<F>::submit_access(prompted_accesses) }

    fn resolve_access<'new>(program_registry: Arc<ProgramRegistry>, derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        let result = Shared::<F>::resolve_access(program_registry, derived_results)?;

        let resource = result.as_ref().to_owned();

        Ok(Owned {
            resource,
            _f: PhantomData::default()
        })
    }
}