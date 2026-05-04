#[derive(Debug, PartialEq)]
pub enum SystemStatus {
    Ready,
    Executing,
    Pending,
    Executed
}