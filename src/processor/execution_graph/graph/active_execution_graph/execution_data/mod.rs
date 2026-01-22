use std::sync::atomic::{AtomicUsize, Ordering};

use crate::prelude::{Status, StatusKind};

pub mod status;

pub struct ExecutionData {
    dependencies: AtomicUsize,
    status: Status
}

impl ExecutionData {
    pub fn new(dependency_count: usize) -> Self {
        Self {
            dependencies: AtomicUsize::new(dependency_count),
            status: Status::default()
        }
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn mark_status(&self, new_status: StatusKind, expected_current_status: Option<StatusKind>) -> Result<StatusKind, StatusKind> {
        self.status.replace(new_status, expected_current_status)
    }

    pub fn mark_dependency(&self, dependency_count: usize) -> Result<usize, usize> {
        self.dependencies.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old_dependency_count| {
            Some(old_dependency_count.checked_sub(dependency_count)?)
        })
    }

    pub fn foo(&self) -> bool {
        self.status.get() != StatusKind::Complete && self.dependencies.load(Ordering::Acquire) == 0
    }
}