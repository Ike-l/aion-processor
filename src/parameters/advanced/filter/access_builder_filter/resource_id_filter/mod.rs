use std::marker::PhantomData;

use aion_program::prelude::{AccessBuilder, DerivedResult};

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

    fn test_derived_result(derived_result: &DerivedResult) -> bool {
        derived_result.resource_id() == Some(&R::get_resource_id())
    }
}
