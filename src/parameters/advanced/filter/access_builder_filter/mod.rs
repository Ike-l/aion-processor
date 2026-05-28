use aion_program::prelude::{AccessBuilder, DerivedError, ResolvedResource};

pub mod user_details_filter;
pub mod resource_id_filter;

pub trait AccessBuilderFilter {
    fn test_access_builder(access_builder: &AccessBuilder) -> bool;
    fn test_derived_result(derived_result: &Result<ResolvedResource<'_>, DerivedError>) -> bool;
}