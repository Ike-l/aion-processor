use crate::prelude::sync::Arc;

pub struct DumbWaker;

impl std::task::Wake for DumbWaker {
    fn wake(self: Arc<Self>) {}
}