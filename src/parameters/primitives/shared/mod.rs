use std::any::TypeId;

use aion_program::prelude::{CastedResource, Injection, AccessBuilder, FinalisedAccess, DerivedResult, ResolveResourceError, ResourceAccess, ResourceId, AccessSubmissionError, ResolvedResource};

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

    fn claim_indexes(_access_builders: Vec<&AccessBuilder>) -> Vec<usize> { vec![] }

    fn submit_access(mut prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> {
        if prompted_accesses.len() > 1 {
            return Err(AccessSubmissionError::TooManyPrompts)
        } else if prompted_accesses.len() < 1 {
            return Err(AccessSubmissionError::NotEnoughPrompts)
        }

        let mut access_builder = prompted_accesses.remove(0);
        access_builder.resource_access.replace(ResourceAccess::Shared(1));
        if access_builder.resource_id.is_none() {
            access_builder.resource_id.replace(ResourceId::TypeId(TypeId::of::<T>()));
        }

        Ok(vec![access_builder.build().unwrap()])
    }

    fn resolve_access<'new>(mut derived_results: Vec<DerivedResult<'new>>) -> Result<Self::Item<'new>, ResolveResourceError> {
        if derived_results.len() > 1 {
            return Err(ResolveResourceError::TooManyResults)
        } else if derived_results.len() < 1 {
            return Err(ResolveResourceError::NotEnoughResults)
        }

        let derived_result = derived_results.pop().unwrap();

        let resolved_resource: ResolvedResource = derived_result.try_into()?;
        let casted_resource = resolved_resource.cast::<T>().map_err(|_| ResolveResourceError::Casting)?;

        Ok(Shared { resource: casted_resource })
    }
}
