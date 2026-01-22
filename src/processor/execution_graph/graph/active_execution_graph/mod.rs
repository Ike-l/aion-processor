use std::{collections::HashMap, hash::Hash};

use crate::prelude::{ExecutionData, ExecutionGraph, NodeMap, Status, StatusKind};

pub mod execution_data;

pub struct ActiveExecutionGraph<T> {
    nodes: NodeMap<T>,
    currently_executing: HashMap<T, ExecutionData>
}

impl<T: Hash + Eq + Clone> From<ExecutionGraph<T>> for ActiveExecutionGraph<T> {
    fn from(value: ExecutionGraph<T>) -> Self {
        let currently_executing = value.nodes.create_execution_flow().collect();
        Self {
            nodes: value.nodes,
            currently_executing
        }
    }
}

impl<T: Hash + Eq> ActiveExecutionGraph<T> {
    pub fn get_leaves(&self) -> impl Iterator<Item = (&T, &Status)> {
        self.currently_executing
            .iter()
            .filter_map(|(data, execution_data)| {
                if execution_data.foo() {
                    Some((data, execution_data.status()))
                } else {
                    None
                }
            })
    }

    pub fn mark_status(&self, target: &T, new_status: StatusKind) -> Option<Result<StatusKind, StatusKind>> {
        match new_status {
            StatusKind::Incomplete => panic!("Cannot set as Incomplete"),
            StatusKind::Pending => {
                Some(self.currently_executing
                    .get(target)?
                    .mark_status(
                        new_status, 
                        Some(StatusKind::Incomplete)
                    )
                )
            },
            StatusKind::Complete => {
                let old_status = self.currently_executing
                    .get(target)?
                    .mark_status(
                        new_status, 
                        None
                    ).unwrap();

                if old_status == StatusKind::Complete {
                    return Some(Ok(old_status));
                }

                for dependent_node in self.nodes.find_all_dependent(target) {
                    let _ = self.currently_executing.get(dependent_node).unwrap().mark_dependency(1);
                }

                Some(Ok(old_status))
            },
        }
    }
}