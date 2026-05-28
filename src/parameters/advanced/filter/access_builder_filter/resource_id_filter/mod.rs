use std::marker::PhantomData;

use aion_program::prelude::{AccessBuilder, DerivedError, ResolvedResource};

use crate::prelude::AccessBuilderFilter;

pub trait ResourceIdGetter {
    fn get_resource_id() -> Option<aion_program::prelude::ResourceId>;
}

pub struct ResourceIdFilter<R: ResourceIdGetter> {
    _u: PhantomData<R>
}

impl<R: ResourceIdGetter> AccessBuilderFilter for ResourceIdFilter<R> {
    fn test_access_builder(access_builder: &AccessBuilder) -> bool {
        access_builder.resource_id == R::get_resource_id()
    }

    fn test_derived_result(derived_result: &Result<ResolvedResource<'_>, DerivedError>) -> bool {
        derived_result.as_ref().is_ok_and(|resolved_resource| resolved_resource.resource_id() == &R::get_resource_id())
    }
}
