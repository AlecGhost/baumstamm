use baumstamm_lib::{
    graph::{person_layers, Graph},
    FamilyTree,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;

type Pid = baumstamm_lib::PersonId;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub enum GridItem {
    Person(Pid),
    Connections(Connections),
}

impl Default for GridItem {
    fn default() -> Self {
        Self::Connections(Connections::default())
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Type)]
pub struct Connections {
    orientation: Orientation,
    total: u32,
    passing: Vec<Passing>,
    ending: Vec<Ending>,
    crossing: Vec<Crossing>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Type)]
pub enum Orientation {
    #[default]
    Up,
    Down,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Passing {
    connection: u32,
    color: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Ending {
    connection: u32,
    color: u32,
    origin: Origin,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Crossing {
    connection: u32,
    color: u32,
    origin: Origin,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub enum Origin {
    Left,
    Right,
    None,
}

#[derive(Clone, Debug, Default)]
struct RelIndices {
    parents: [Option<usize>; 2],
    children: Vec<usize>,
    crossing_point: Option<usize>,
}

pub fn generate(tree: &FamilyTree) -> Vec<Vec<GridItem>> {
    let rels = tree.get_relationships();
    let layers = Graph::new(rels).cut().layers();
    let person_layers = person_layers(&layers, rels);
    let length = person_layers
        .iter()
        .map(|layer| layer.len())
        .max()
        .unwrap_or_default();
    if length == 0 {
        return Vec::new();
    }
    let person_indices = person_layers
        .iter()
        .map(|layer| {
            let start_index = middle(layer.len(), length);
            layer
                .iter()
                .enumerate()
                .map(|(i, pid)| (start_index + i, pid))
                .collect_vec()
        })
        .collect_vec();
    eprintln!("person_indices: {person_indices:#?}");
    let mut rel_indices = layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            layer
                .iter()
                .map(|rid| {
                    let rel = rels
                        .iter()
                        .find(|rel| rel.id == *rid)
                        .expect("Inconsistent relationships");
                    eprintln!("rel: {rel:#?}");
                    let mut rel_indices = RelIndices::default();
                    if index > 0 {
                        if let Some(parent_indices) = person_indices.get(index - 1) {
                            let mut parent_indices = rel.parents.map(|opt_parent| {
                                opt_parent
                                    .map(|parent| {
                                        parent_indices
                                            .iter()
                                            .find(|(_, pid)| **pid == parent)
                                            .map(|(i, _)| *i)
                                    })
                                    .flatten()
                            });
                            parent_indices.sort();
                            rel_indices.parents = parent_indices;

                            // crossing point is only necessary, if there are children
                            if !rel.children.is_empty() {
                                rel_indices.crossing_point = match parent_indices {
                                    [Some(a), Some(b)] => Some(a + middle(a, b)),
                                    [Some(a), None] => Some(a),
                                    [None, Some(b)] => Some(b),
                                    [None, None] => None,
                                }
                            }
                        }
                    }
                    if let Some(child_indices) = person_indices.get(index) {
                        let children_indices = rel
                            .children
                            .iter()
                            .filter_map(|child| child_indices.iter().find(|(_, pid)| *pid == child))
                            .map(|(i, _)| *i)
                            .sorted()
                            .collect_vec();
                        if children_indices.len() == rel.children.len() {
                            rel_indices.children = children_indices;
                        }
                    }
                    eprintln!("rel_indices: {rel_indices:#?}");
                    rel_indices
                })
                .collect_vec()
        })
        .collect_vec();
    rel_indices.push(Vec::new());
    let mut grid = vec![Vec::new(); person_layers.len() * 3];
    for (layer_index, layer) in grid.iter_mut().enumerate() {
        for item_index in 0..length {
            let item = match layer_index % 3 {
                0 => {
                    // sibling row
                    let rel_indices = &rel_indices[layer_index / 3];
                    let mut connections = Connections {
                        orientation: Orientation::Down,
                        total: rel_indices
                            .len()
                            .try_into()
                            .expect("Too many relationships"),
                        ..Default::default()
                    };
                    rel_indices
                        .iter()
                        .enumerate()
                        .filter(|(_, rel_indices)| match rel_indices.children.len() {
                            0 => false,
                            1 if rel_indices.crossing_point.is_none() => false,
                            _ => true,
                        })
                        .for_each(|(connection, rel_indices)| {
                            let first = *rel_indices
                                .children
                                .first()
                                .expect("Must contain first child");
                            let last = *rel_indices
                                .children
                                .last()
                                .expect("Must contain last child");
                            assert!(first <= last, "Unsorted children");
                            let start = rel_indices
                                .crossing_point
                                .map(|point| point.min(first))
                                .unwrap_or(first);
                            let end = rel_indices
                                .crossing_point
                                .map(|point| point.max(last))
                                .unwrap_or(last);
                            if let Some(crossing_point) = rel_indices.crossing_point {
                                if item_index == crossing_point {
                                    let origin = if crossing_point < first {
                                        Origin::Right
                                    } else if crossing_point > last {
                                        Origin::Left
                                    } else {
                                        Origin::None
                                    };
                                    connections.crossing.push(Crossing {
                                        connection: connection
                                            .try_into()
                                            .expect("Too many relationships"),
                                        color: 0,
                                        origin,
                                    });
                                    if rel_indices.children.len() == 1 && item_index == first {
                                        connections.ending.push(Ending {
                                            connection: connection
                                                .try_into()
                                                .expect("Too many relationships"),
                                            color: 0,
                                            origin: Origin::None,
                                        });
                                        return;
                                    }
                                }
                            }
                            if item_index == start && item_index == first {
                                connections.ending.push(Ending {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                    origin: Origin::Right,
                                });
                            } else if item_index == end && item_index == last {
                                connections.ending.push(Ending {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                    origin: Origin::Left,
                                });
                            } else if start < item_index && item_index < end {
                                if rel_indices
                                    .children
                                    .iter()
                                    .any(|index| *index == item_index)
                                {
                                    connections.ending.push(Ending {
                                        connection: connection
                                            .try_into()
                                            .expect("Too many relationships"),
                                        color: 0,
                                        origin: Origin::None,
                                    });
                                }
                                connections.passing.push(Passing {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                })
                            }
                        });
                    GridItem::Connections(connections)
                }
                1 => {
                    // person row
                    person_indices[(layer_index - 1) / 3]
                        .iter()
                        .find(|(i, _)| *i == item_index)
                        .map(|(_, pid)| GridItem::Person(**pid))
                        .unwrap_or_default()
                }
                2 => {
                    // relationship row
                    let rel_indices = &rel_indices[(layer_index - 2) / 3 + 1];
                    let mut connections = Connections {
                        orientation: Orientation::Up,
                        total: rel_indices
                            .len()
                            .try_into()
                            .expect("Too many relationships"),
                        ..Default::default()
                    };
                    rel_indices
                        .iter()
                        .enumerate()
                        .filter(|(_, rel_indices)| !matches!(rel_indices.parents, [None, None]))
                        .for_each(|(connection, rel_indices)| {
                            let first = rel_indices.parents[0];
                            let last = rel_indices.parents[1];
                            assert!(first <= last, "Unsorted parents");
                            if let Some(crossing_point) = rel_indices.crossing_point {
                                if item_index == crossing_point {
                                    connections.crossing.push(Crossing {
                                        connection: connection
                                            .try_into()
                                            .expect("Too many relationships"),
                                        color: 0,
                                        origin: Origin::None,
                                    });
                                }
                            }
                            if matches!(first, Some(index) if index == item_index) {
                                let origin = if last.is_some() {
                                    Origin::Right
                                } else {
                                    Origin::None
                                };
                                connections.ending.push(Ending {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                    origin,
                                });
                            } else if matches!(last, Some(index) if index == item_index) {
                                let origin = if first.is_some() {
                                    Origin::Left
                                } else {
                                    Origin::None
                                };
                                connections.ending.push(Ending {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                    origin,
                                });
                            } else if matches!((first, last),
                                (Some(start), Some(end)) if start < item_index && item_index < end)
                            {
                                connections.passing.push(Passing {
                                    connection: connection
                                        .try_into()
                                        .expect("Too many relationships"),
                                    color: 0,
                                })
                            }
                        });
                    GridItem::Connections(connections)
                }
                _ => panic!("Math broken"),
            };
            layer.push(item);
        }
    }
    grid
}

fn middle(a: usize, b: usize) -> usize {
    let diff = if a <= b { b - a } else { a - b };
    if diff % 2 == 0 {
        diff / 2
    } else {
        (diff - 1) / 2
    }
}
