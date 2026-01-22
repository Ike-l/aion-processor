use std::collections::HashSet;

use crate::prelude::{Ordering, SystemId};

pub struct ProcessorOrdering {
    assert_before: HashSet<SystemId>,
    assert_after: HashSet<SystemId>,

    end_of_chain_priority: f64,
}

impl Ordering for ProcessorOrdering {
    type Item = SystemId;

    fn constrained_by(&self, superset: &HashSet<Self::Item>) -> Self {
        Self {
            assert_before: self.assert_before.intersection(superset).cloned().collect(),
            assert_after: self.assert_after.intersection(superset).cloned().collect(),
            end_of_chain_priority: self.end_of_chain_priority
        }
    }

    fn into_assert_before_assert_after_priority(self) ->
            ( HashSet<Self::Item>, HashSet<Self::Item>, f64) {
        let Self {
            assert_before,
            assert_after,
            end_of_chain_priority    
        } = self;

        (assert_before, assert_after, end_of_chain_priority)
    }
}