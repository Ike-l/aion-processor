use std::sync::Arc;

use threadpool::ThreadPool;
use tokio::runtime::Runtime;

pub struct ProcessConfig<'a> {
    pub runtime: Arc<Option<Runtime>>,
    pub threadpool: Option<&'a ThreadPool>,
}