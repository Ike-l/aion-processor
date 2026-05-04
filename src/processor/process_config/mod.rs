use threadpool::ThreadPool;
use tokio::runtime::Runtime;

pub struct ProcessConfig<'a> {
    pub runtime: Option<&'a Runtime>,
    pub threadpool: Option<&'a ThreadPool>,
    pub collision_check: bool,   
}