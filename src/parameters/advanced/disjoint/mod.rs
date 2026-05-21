use std::{marker::PhantomData, sync::Arc};

use hecs::Entity;

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError};

pub struct Disjoint<'a, A, B, C: Injection> {
    pub resource: C::Item<'a>,
    _a: PhantomData<A>,
    _b: PhantomData<B>,
}

impl<'a, A: Injection, B: Injection, C: Injection> Injection for Disjoint<'a, A, B, C> {
    type Item<'new> = Disjoint<'new, A, B, C>;

    fn claim_manual_access_builders(accesses: Vec<&AccessBuilder>) -> Vec<usize> { A::claim_manual_access_builders(accesses) }

    fn submit_access(prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> { B::submit_access(prompted_accesses) }

    fn resolve_access<'new>(entity: Option<Entity>, program_registry: Arc<ProgramRegistry>, derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        let resource = C::resolve_access(entity, program_registry, derived_results)?;

        Ok(Disjoint {
            resource,
            _a: PhantomData::default(),
            _b: PhantomData::default()
        })
    }
}