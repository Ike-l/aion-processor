use std::any::TypeId;

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum SystemId {
    Label(String),
    TypeId(TypeId)
}