use std::any::TypeId;

use aion_program::prelude::{CastedResource, Injection, AccessBuilder, FinalisedAccess, DerivedResult, ResolveResourceError, ResourceAccess, ResourceId, AccessSubmissionError, ResolvedResource};

pub struct Unique<'a, T> {
    resource: CastedResource<'a, T>
}

impl<'a, T> Unique<'a, T> {
    pub fn as_ref(&self) -> &T {
        self.resource.as_ref().expect("Expected Unique Resource")
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.resource.as_mut().expect("Expected Unique Resource")
    }
}

impl<'a, T: 'static> Injection for Unique<'a, T> {
    type Item<'new> = Unique<'new, T>;

    fn submit_access(mut prompted_accesses: Vec<AccessBuilder>) -> Result<Vec<FinalisedAccess>, AccessSubmissionError> {
        if prompted_accesses.len() > 1 {
            return Err(AccessSubmissionError::TooManyPrompts)
        } else if prompted_accesses.len() < 1 {
            return Err(AccessSubmissionError::NotEnoughPrompts)
        }

        let mut access_builder = prompted_accesses.pop().unwrap();
        access_builder.resource_access.replace(ResourceAccess::Unique);
        access_builder.resource_id.replace(ResourceId::TypeId(TypeId::of::<T>()));

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

        Ok(Unique { resource: casted_resource })
    }
}
