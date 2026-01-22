
use std::collections::HashSet;

pub trait Ordering {
    type Item;

    fn constrained_by(&self, superset: &HashSet<Self::Item>) -> Self;
    fn into_assert_before_assert_after_priority(self) ->
        ( HashSet<Self::Item>, HashSet<Self::Item>, f64);
}