use std::marker::PhantomData;

use aion_program::prelude::{AccessBuilder, DerivedResult, UserId, UserPassword};

use crate::prelude::AccessBuilderFilter;

pub trait UserDetailsGetter {
    fn get_user_details() -> Option<(UserId, UserPassword)>;
}

pub struct UserDetailsFilter<U: UserDetailsGetter> {
    _u: PhantomData<U>
}

impl<U: UserDetailsGetter> AccessBuilderFilter for UserDetailsFilter<U> {
    fn test_access_builder(access_builder: &AccessBuilder) -> bool {
        access_builder.user_details == U::get_user_details()
    }

    fn test_derived_result(derived_result: &DerivedResult) -> bool {
        derived_result.user_details() == Some(&U::get_user_details())
    }
}



