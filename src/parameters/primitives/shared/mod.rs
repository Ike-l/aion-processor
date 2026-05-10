use std::{any::TypeId, sync::Arc};

use aion_program::prelude::{AccessBuilder, AccessSubmissionError, CastedResource, DerivedResult, FinalisedAccess, Injection, ProgramRegistry, ResolveResourceError, ResolvedResource, ResourceAccess, ResourceId};

pub struct Shared<'a, T> {
    resource: CastedResource<'a, T>
}

impl<'a, T> Shared<'a, T> {
    pub fn as_ref(&self) -> &T {
        self.resource.as_ref().expect("Expected Shared Resource")
    }
}

impl<'a, T: 'static> Injection for Shared<'a, T> {
    type Item<'new> = Shared<'new, T>;

    fn claim_manual_access_builders(_access_builders: Vec<&AccessBuilder>) -> Vec<usize> { vec![] }

    fn submit_access(mut prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> {
        if prompted_accesses.len() == 0 {
            return Ok(vec![
                AccessBuilder {
                    resource_id: Some(ResourceId::TypeId(TypeId::of::<T>())),
                    resource_access: Some(ResourceAccess::Shared(1)),
                    ..Default::default()
                }.build().unwrap()
            ])
        }

        let mut access_builder = prompted_accesses.remove(0);
        access_builder.resource_access.replace(ResourceAccess::Shared(1));
        if access_builder.resource_id.is_none() {
            access_builder.resource_id.replace(ResourceId::TypeId(TypeId::of::<T>()));
        }

        Ok(vec![access_builder.build().unwrap()])
    }

    fn resolve_access<'new>(_program_registry: Arc<ProgramRegistry>, mut derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        if derived_results.len() < 1 {
            return Err(ResolveResourceError::NotEnoughResults)
        }

        let derived_result = derived_results.pop().unwrap();

        let resolved_resource: ResolvedResource = derived_result.try_into()?;
        let casted_resource = resolved_resource.cast::<T>().map_err(|_| ResolveResourceError::Casting)?;

        Ok(Shared { resource: casted_resource })
    }
}
