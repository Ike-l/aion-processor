use std::{collections::HashSet, hash::Hash, rc::Rc};

pub struct Node<T> {
    parent: Option<Rc<Self>>,
    data: T
}

impl<T> Node<T> where T: Eq + Hash + Clone {
    pub fn new(parent: Option<Rc<Self>>, data: T) -> Self {
        Self {
            parent, data
        }
    }

    pub fn parent(&self) -> &Option<Rc<Self>> {
        &self.parent
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn get_data_up(&self) -> HashSet<T> {
        let mut data = HashSet::new();

        let mut current = self;
        while current.parent.is_some() {
            data.insert(current.data.clone());
            current = current.parent.as_ref().unwrap();
        }

        data.insert(current.data.clone());

        data
    }
}