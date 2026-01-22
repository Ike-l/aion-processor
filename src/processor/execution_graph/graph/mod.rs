use std::{collections::{HashMap, HashSet}, hash::Hash, rc::Rc};

use crate::prelude::{Node, NodeMap, Ordering};

pub mod node;
pub mod node_map;
pub mod active_execution_graph;

pub struct ExecutionGraph<T> {
    nodes: NodeMap<T>,
    // data_world: HashSet<T>
}

impl<T> Default for ExecutionGraph<T> {
    fn default() -> Self {
        Self { 
            nodes: NodeMap::default(),
            // data_world: HashSet::new()
        }
    }
}

impl<T: Hash + Eq + Clone> ExecutionGraph<T> {
    fn flatten_befores_into_afters<Order: Ordering<Item = T>>(nodes: impl Iterator<Item = (T, Order)>) -> HashMap<T, (HashSet<T>, f64)> {
        let mut node_assert_afters_map = HashMap::new();
        let mut node_priority_map = HashMap::new();

        for (data, order) in nodes {
            let (
                assert_befores,
                assert_afters,
                priority
            ) = order.into_assert_before_assert_after_priority();

            for before in assert_befores {
                node_assert_afters_map
                    .entry(before)
                    .or_insert(HashSet::new())
                    .insert(data.clone());
            }

            node_assert_afters_map
                .entry(data.clone())
                .or_insert(HashSet::new())
                .extend(assert_afters);

            node_priority_map.insert(data, priority);
        }

        let mut new_nodes = HashMap::new();
        for (data, priority) in node_priority_map.drain() {
            let assert_afters = node_assert_afters_map.remove(&data).unwrap();
            new_nodes.insert(data, (assert_afters, priority));
        }
        
        new_nodes
    }

    pub fn break_cycles(
        nodes: &mut HashMap<T, (HashSet<T>, f64)>,
        data_world: &HashSet<T>
    ) {
        let mut seen = HashSet::new();

        Self::construct_paths(nodes, None, &mut seen);

        while seen != *data_world {
            if let Some((data, _)) = {
                nodes
                    .iter()
                    .filter(|(data, _)| {
                        !seen.contains(&data)
                    })
                    .max_by(|(_, (_, p1)), (_, (_, p2))| {
                        p1.total_cmp(p2)
                    })
            } {
                let current_path = Node::new(None, (*data).clone());

                seen.insert((*data).clone());

                let current_path = Rc::new(current_path);

                Self::construct_paths(nodes, Some(Rc::clone(&current_path)), &mut seen);
            } else {
                unreachable!("this case implies `seen == node_list`");
            }
        }
    }

    pub fn construct_paths(
        nodes: &mut HashMap<T, (HashSet<T>, f64)>, 
        current_path: Option<Rc<Node<T>>>, 
        seen_list: &mut HashSet<T>
    ) {
        let leaves: HashSet<T> = match &current_path {
            Some(current_path) => {
                nodes.iter().filter_map(|(data, (after, _))| {
                    if after.contains(current_path.data()) {
                        Some(data)
                    } else {
                        None
                    }
                }).cloned().collect()
            },
            None => {
                nodes.iter().filter_map(|(data, (after, _))| {
                    if after.is_empty() {
                        Some(data)
                    } else {
                        None
                    }
                }).cloned().collect()
            }
        };

        for leaf in leaves {
            seen_list.insert(leaf.clone());

            if let Some((max, child_of_max)) = Self::find_max_priority_in_cycle(nodes, &current_path, &leaf) {
                if let Some(child_of_max) = child_of_max {
                    nodes.get_mut(child_of_max).unwrap().0.remove(max);
                } else {
                    nodes.get_mut(&leaf).unwrap().0.remove(max);
                }
            } else {
                let new_current_path = if let Some(current_path) = &current_path {
                    Node::new(Some(Rc::clone(current_path)), leaf)
                } else {
                    Node::new(None, leaf)
                };

                Self::construct_paths(nodes, Some(Rc::new(new_current_path)), seen_list);
            }
        }
    }

    pub fn find_max_priority_in_cycle<'a>(
        priority_mapping: &HashMap<T, (HashSet<T>, f64)>, 
        current_path: &'a Option<Rc<Node<T>>>,
        looking_for: &T
    ) -> Option<(&'a T, Option<&'a T>)> {
        if current_path.is_none() {
            return None
        }

        let current_path = current_path.as_ref().unwrap();

        let mut child_of_max = None;
        let mut max = current_path;

        #[allow(unused_assignments)]
        let mut child_of_current = None;
        
        let mut current = current_path;

        while current.parent().is_some() {
            child_of_current = Some(current);
            current = current.parent().as_ref().unwrap();

            let max_priority = priority_mapping.get(max.data()).unwrap().1;
            let current_priority = priority_mapping.get(current.data()).unwrap().1;

            if current_priority > max_priority {
                child_of_max = child_of_current;
                max = current;
            }

            if *current.data() == *looking_for {
                return Some((max.data(), child_of_max.map(|node| node.data())))
            }
        }

        None
    }

    pub fn new<Order: Ordering<Item = T>>(nodes: &[(T, &Order)]) -> Self {
        if nodes.len() == 0 {
            return Self::default();
        }

        let data_world = nodes
            .iter()
            .map(|(data, _)| data.clone())
            .collect::<HashSet<_>>();

        let constrained_nodes = nodes
            .into_iter()
            .map(|(data, order)| {
                (data.clone(), order.constrained_by(&data_world))
            });

        let mut nodes = Self::flatten_befores_into_afters(constrained_nodes);

        Self::break_cycles(&mut nodes, &data_world);

        let nodes = nodes.into_iter().map(|(data, (assert_afters, _))| (data, assert_afters));
        let nodes = NodeMap::new(nodes.collect());

        Self {
            nodes,
            // data_world,
        }
    }
}