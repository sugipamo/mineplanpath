use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub edge_id: i64,
    pub edge_name: String,
    pub previous: String,
    pub next: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskSegment {
    pub edge_name: String,
    pub sequence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundPath {
    pub turns: usize,
    pub tasks: Vec<TaskSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct State {
    node: String,
    edge_name: String,
}

#[derive(Debug, Clone)]
struct Step {
    prior: State,
    edge: Edge,
}

pub fn find_path(edges: &[Edge], from: &str, to: &str) -> Option<FoundPath> {
    if from == to {
        return Some(FoundPath {
            turns: 0,
            tasks: Vec::new(),
        });
    }
    let mut adjacency: HashMap<&str, Vec<&Edge>> = HashMap::new();
    for edge in edges {
        adjacency.entry(&edge.previous).or_default().push(edge);
        adjacency.entry(&edge.next).or_default().push(edge);
    }
    for incident in adjacency.values_mut() {
        incident.sort_by_key(|edge| edge.edge_id);
    }
    let initial_names: Vec<String> = adjacency
        .get(from)?
        .iter()
        .map(|edge| edge.edge_name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut initial_names = initial_names;
    initial_names.sort();

    let mut distance: HashMap<State, usize> = HashMap::new();
    let mut prior: HashMap<State, Step> = HashMap::new();
    let mut queue = VecDeque::new();
    for edge_name in initial_names {
        let state = State {
            node: from.into(),
            edge_name,
        };
        distance.insert(state.clone(), 0);
        queue.push_back(state);
    }

    let mut destination = None;
    while let Some(state) = queue.pop_front() {
        let current_distance = distance[&state];
        if state.node == to {
            destination = Some(state);
            break;
        }
        for edge in adjacency.get(state.node.as_str()).into_iter().flatten() {
            let next_node = if edge.previous == state.node {
                &edge.next
            } else {
                &edge.previous
            };
            let turn = usize::from(edge.edge_name != state.edge_name);
            let candidate = current_distance + turn;
            let next_state = State {
                node: next_node.clone(),
                edge_name: edge.edge_name.clone(),
            };
            if distance
                .get(&next_state)
                .is_some_and(|known| *known <= candidate)
            {
                continue;
            }
            distance.insert(next_state.clone(), candidate);
            prior.insert(
                next_state.clone(),
                Step {
                    prior: state.clone(),
                    edge: (*edge).clone(),
                },
            );
            if turn == 0 {
                queue.push_front(next_state);
            } else {
                queue.push_back(next_state);
            }
        }
    }

    let destination = destination?;
    let turns = distance[&destination];
    let mut state = destination;
    let mut traversed = Vec::new();
    while let Some(step) = prior.get(&state) {
        traversed.push((
            step.prior.node.clone(),
            state.node.clone(),
            step.edge.clone(),
        ));
        state = step.prior.clone();
    }
    traversed.reverse();

    let mut tasks: Vec<TaskSegment> = Vec::new();
    for (enter, exit, edge) in traversed {
        match tasks.last_mut() {
            Some(task) if task.edge_name == edge.edge_name => task.sequence.push(exit),
            _ => tasks.push(TaskSegment {
                edge_name: edge.edge_name,
                sequence: vec![enter, exit],
            }),
        }
    }
    Some(FoundPath { turns, tasks })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(id: i64, name: &str, previous: &str, next: &str) -> Edge {
        Edge {
            edge_id: id,
            edge_name: name.into(),
            previous: previous.into(),
            next: next.into(),
        }
    }

    #[test]
    fn compresses_same_named_edges_into_one_task() {
        let result = find_path(
            &[
                edge(1, "x", "A", "B"),
                edge(2, "x", "B", "C"),
                edge(3, "y", "C", "D"),
                edge(4, "y", "D", "E"),
                edge(5, "z", "E", "F"),
            ],
            "A",
            "F",
        )
        .unwrap();
        assert_eq!(result.turns, 2);
        assert_eq!(result.tasks.len(), 3);
        assert_eq!(result.tasks[0].sequence, ["A", "B", "C"]);
        assert_eq!(result.tasks[1].sequence, ["C", "D", "E"]);
        assert_eq!(result.tasks[2].sequence, ["E", "F"]);
    }

    #[test]
    fn traverses_ordered_edges_in_either_direction() {
        let result = find_path(&[edge(1, "x", "A", "B")], "B", "A").unwrap();
        assert_eq!(result.turns, 0);
        assert_eq!(result.tasks[0].sequence, ["B", "A"]);
    }

    #[test]
    fn chooses_fewer_turns_over_fewer_edges() {
        let result = find_path(
            &[
                edge(1, "x", "A", "B"),
                edge(2, "y", "B", "D"),
                edge(3, "road", "A", "C"),
                edge(4, "road", "C", "E"),
                edge(5, "road", "E", "D"),
            ],
            "A",
            "D",
        )
        .unwrap();
        assert_eq!(result.turns, 0);
        assert_eq!(
            result.tasks,
            [TaskSegment {
                edge_name: "road".into(),
                sequence: vec!["A".into(), "C".into(), "E".into(), "D".into()],
            }]
        );
    }

    #[test]
    fn same_node_needs_no_task() {
        assert_eq!(find_path(&[], "A", "A").unwrap().tasks, []);
    }
}
