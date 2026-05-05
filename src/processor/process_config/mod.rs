use std::sync::Arc;

use threadpool::ThreadPool;
use tokio::runtime::Runtime;

pub struct ProcessConfig<'a> {
    pub runtime: Option<&'a Arc<Runtime>>,
    pub threadpool: Option<&'a ThreadPool>,
}