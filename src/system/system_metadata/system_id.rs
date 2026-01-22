#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct SystemId(String);

impl<T: Into<String>> From<T> for SystemId {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}