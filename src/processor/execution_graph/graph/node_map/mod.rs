use std::{collections::{HashMap, HashSet}, hash::Hash};

use crate::prelude::ExecutionData;

pub struct NodeMap<T> {
    // Current, Dependencies
    nodes: HashMap<T, HashSet<T>>
}

impl<T> NodeMap<T> {
    pub fn new(nodes: HashMap<T, HashSet<T>>) -> Self {
        Self { nodes }
    }

}

impl<T: Clone> NodeMap<T> {
    pub fn create_execution_flow(&self) -> impl Iterator<Item = (T, ExecutionData)> {
        self.nodes.iter().map(|(data, dependencies)| {
            (data.clone(), ExecutionData::new(dependencies.len()))
        })
    }
}

impl<T: Hash + Eq> NodeMap<T> {
    pub fn find_all_dependent(&self, target: &T) -> impl Iterator<Item = &T> {
        self.nodes.iter().filter_map(|(data, dependents)| {
            if dependents.contains(target) {
                Some(data)
            } else {
                None
            }
        })
    }
}

impl<T> Default for NodeMap<T> {
    fn default() -> Self {
        Self {
            nodes: HashMap::new()
        }
    }
}