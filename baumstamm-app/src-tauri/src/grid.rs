use std::collections::HashMap;

use baumstamm_lib::{graph::Graph, FamilyTree, Relationship};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;

type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;
type Color = (f32, f32, f32);
type Grid<T> = Vec<Vec<T>>;

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
    total_x: u32,
    total_y: u32,
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
    color: Color,
    y_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Ending {
    connection: u32,
    color: Color,
    origin: Origin,
    x_index: u32,
    y_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Crossing {
    connection: u32,
    color: Color,
    origin: Origin,
    x_index: u32,
    y_index: u32,
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

#[derive(Clone, Debug)]
struct PersonIndex {
    index: usize,
    pid: Pid,
}

#[derive(Clone, Debug, Default)]
struct LineAllocator {
    ending_points: Vec<usize>,
    allocated: HashMap<u32, usize>,
}

impl LineAllocator {
    fn alloc(&mut self, connection: u32, start: usize, end: usize) -> usize {
        use std::collections::hash_map::Entry;
        match self.allocated.entry(connection) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                for (i, ending_point) in self.ending_points.iter_mut().enumerate() {
                    if start > *ending_point {
                        *ending_point = end;
                        entry.insert(i);
                        return i;
                    }
                }
                self.ending_points.push(end);
                let index = self.ending_points.len() - 1;
                entry.insert(index);
                index
            }
        }
    }

    fn total(&self) -> usize {
        self.ending_points.len()
    }
}

pub fn generate(tree: &FamilyTree) -> Grid<GridItem> {
    let rels = tree.get_relationships();
    let graph = Graph::new(rels).cut();
    let layers = graph.layers();
    let person_layers = graph.person_layers(rels);
    let row_length = person_layers
        .iter()
        .map(|layer| layer.len())
        .max()
        .unwrap_or_default();
    if row_length == 0 {
        return Vec::new();
    }
    let person_indices = person_layers
        .iter()
        .map(|layer| {
            let start_index = middle(layer.len(), row_length);
            layer
                .iter()
                .enumerate()
                .map(|(i, pid)| PersonIndex {
                    index: start_index + i,
                    pid: *pid,
                })
                .collect_vec()
        })
        .collect_vec();
    let rel_indices = get_rel_indices(&layers, rels, &person_indices);

    fill_grid(&person_indices, &rel_indices, row_length)
}

/// Fill grid with `GridItem`s
fn fill_grid(
    person_indices: &Grid<PersonIndex>,
    rel_indices: &Grid<RelIndices>,
    row_length: usize,
) -> Grid<GridItem> {
    let mut grid: Grid<GridItem> = Vec::new();
    for layer_index in 0..(person_indices.len() * 3) {
        let mut layer = Vec::new();
        let mut line_allocator = LineAllocator::default();
        for item_index in 0..row_length {
            let item = match layer_index % 3 {
                0 => new_sibling_item(
                    rel_indices[layer_index / 3].as_slice(),
                    item_index,
                    get_rel_connections(&mut grid, layer_index, item_index),
                    |connection, start, end| {
                        line_allocator
                            .alloc(connection, start, end)
                            .try_into()
                            .expect("Too many relationships")
                    },
                ),
                1 => new_person_item(person_indices[(layer_index - 1) / 3].as_slice(), item_index),
                2 => new_relationship_item(
                    rel_indices[(layer_index - 2) / 3 + 1].as_slice(),
                    item_index,
                    |connection, start, end| {
                        line_allocator
                            .alloc(connection, start, end)
                            .try_into()
                            .expect("Too many relationships")
                    },
                ),
                _ => panic!("Math broken"),
            };
            layer.push(item);
        }
        let total_y = line_allocator
            .total()
            .try_into()
            .expect("Too many relationships");
        layer
            .iter_mut()
            .filter_map(|item| match item {
                GridItem::Connections(connection) => Some(connection),
                GridItem::Person(_) => None,
            })
            .for_each(|connection| {
                connection.total_y = total_y;
            });
        grid.push(layer);
    }
    grid
}

fn get_rel_indices(
    layers: &Grid<Rid>,
    rels: &[Relationship],
    person_indices: &Grid<PersonIndex>,
) -> Grid<RelIndices> {
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
                    let mut rel_indices = RelIndices::default();
                    if index > 0 {
                        if let Some(parent_indices) = person_indices.get(index - 1) {
                            let mut parent_indices = rel.parents.map(|opt_parent| {
                                opt_parent.and_then(|parent| {
                                    parent_indices
                                        .iter()
                                        .find(|pi| pi.pid == parent)
                                        .map(|pi| pi.index)
                                })
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
                            .filter_map(|child| child_indices.iter().find(|pi| pi.pid == *child))
                            .map(|pi| pi.index)
                            .sorted()
                            .collect_vec();
                        if children_indices.len() == rel.children.len() {
                            rel_indices.children = children_indices;
                        }
                    }
                    rel_indices
                })
                .collect_vec()
        })
        .collect_vec();
    rel_indices.push(Vec::new());
    rel_indices
}

/// Relationship `Connections` above new sibling item
fn get_rel_connections(
    grid: &mut Grid<GridItem>,
    layer_index: usize,
    item_index: usize,
) -> Option<&mut Connections> {
    if layer_index == 0 {
        None
    } else if let GridItem::Connections(connections) = &mut grid[layer_index - 1][item_index] {
        Some(connections)
    } else {
        None
    }
}

fn new_sibling_item<F>(
    rel_indices: &[RelIndices],
    item_index: usize,
    mut rel_connections: Option<&mut Connections>,
    mut alloc: F,
) -> GridItem
where
    F: FnMut(u32, usize, usize) -> u32,
{
    fn get_x_index(
        connection: u32,
        is_crossing: bool,
        sibling_connections: &mut Connections,
        rel_connections: Option<&mut Connections>,
    ) -> u32 {
        if let Some(rel_connections) = rel_connections {
            if is_crossing {
                rel_connections
                    .crossing
                    .iter()
                    .find(|crossing| crossing.connection == connection)
                    .expect("Must contain crossing")
                    .x_index
            } else {
                rel_connections.total_x += 1;
                ppp(&mut sibling_connections.total_x)
            }
        } else {
            ppp(&mut sibling_connections.total_x)
        }
    }

    let mut connections = Connections {
        orientation: Orientation::Down,
        total_x: rel_connections
            .as_ref()
            .map_or(0, |connections| connections.total_x),
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
        .sorted_by(|a, b| a.1.children.first().cmp(&b.1.children.first()))
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
            let connection = connection.try_into().expect("Too many relationships");
            let start = rel_indices
                .crossing_point
                .map(|point| point.min(first))
                .unwrap_or(first);
            let end = rel_indices
                .crossing_point
                .map(|point| point.max(last))
                .unwrap_or(last);
            let starting_point = if let Some(crossing_point) = rel_indices.crossing_point {
                if crossing_point < start {
                    crossing_point
                } else {
                    start
                }
            } else {
                start
            };
            let ending_point = if let Some(crossing_point) = rel_indices.crossing_point {
                if crossing_point > end {
                    crossing_point
                } else {
                    end
                }
            } else {
                end
            };
            let y_index = alloc(connection, starting_point, ending_point);
            if let Some(crossing_point) = rel_indices.crossing_point {
                if item_index == crossing_point {
                    let origin = if crossing_point < first
                        || crossing_point == first && rel_indices.children.len() > 1
                    {
                        Origin::Right
                    } else if crossing_point > last {
                        Origin::Left
                    } else {
                        Origin::None
                    };
                    let x_index = get_x_index(
                        connection,
                        true,
                        &mut connections,
                        rel_connections.as_deref_mut(),
                    );
                    connections.crossing.push(Crossing {
                        connection,
                        color: color(connection),
                        origin,
                        x_index,
                        y_index,
                    });
                    if rel_indices.children.len() == 1 && item_index == first {
                        // if it's a single child, connect crossing directly to ending
                        connections.ending.push(Ending {
                            connection,
                            color: color(connection),
                            origin: Origin::None,
                            x_index,
                            y_index,
                        });
                        return;
                    }
                }
            }
            if item_index == start && item_index == first {
                let x_index = get_x_index(
                    connection,
                    false,
                    &mut connections,
                    rel_connections.as_deref_mut(),
                );
                connections.ending.push(Ending {
                    connection,
                    color: color(connection),
                    origin: Origin::Right,
                    x_index,
                    y_index,
                });
            } else if item_index == end && item_index == last {
                let x_index = get_x_index(
                    connection,
                    false,
                    &mut connections,
                    rel_connections.as_deref_mut(),
                );
                connections.ending.push(Ending {
                    connection,
                    color: color(connection),
                    origin: Origin::Left,
                    x_index,
                    y_index,
                });
            } else if start < item_index && item_index < end {
                if rel_indices
                    .children
                    .iter()
                    .any(|index| *index == item_index)
                {
                    let x_index = get_x_index(
                        connection,
                        false,
                        &mut connections,
                        rel_connections.as_deref_mut(),
                    );
                    connections.ending.push(Ending {
                        connection,
                        color: color(connection),
                        origin: Origin::None,
                        x_index,
                        y_index,
                    });
                }
                connections.passing.push(Passing {
                    connection,
                    color: color(connection),
                    y_index,
                })
            }
        });
    GridItem::Connections(connections)
}

fn new_person_item(person_indices: &[PersonIndex], item_index: usize) -> GridItem {
    person_indices
        .iter()
        .find(|pi| pi.index == item_index)
        .map(|pi| GridItem::Person(pi.pid))
        .unwrap_or_default()
}

fn new_relationship_item<F>(rel_indices: &[RelIndices], item_index: usize, mut alloc: F) -> GridItem
where
    F: FnMut(u32, usize, usize) -> u32,
{
    let mut connections = Connections {
        orientation: Orientation::Up,
        ..Default::default()
    };
    rel_indices
        .iter()
        .enumerate()
        .filter(|(_, rel_indices)| !matches!(rel_indices.parents, [None, None]))
        .sorted_by(|a, b| a.1.parents.first().cmp(&b.1.parents.first()))
        .for_each(|(connection, rel_indices)| {
            let first = rel_indices.parents[0];
            let last = rel_indices.parents[1];
            assert!(first <= last, "Unsorted parents");
            let connection = connection.try_into().expect("Too many relationships");
            let y_index = {
                let start = if let Some(crossing_point) = rel_indices.crossing_point {
                    if let Some(start) = first {
                        if crossing_point < start {
                            Some(crossing_point)
                        } else {
                            first
                        }
                    } else {
                        Some(crossing_point)
                    }
                } else {
                    first
                };
                let end = if let Some(crossing_point) = rel_indices.crossing_point {
                    if let Some(end) = last {
                        if crossing_point > end {
                            Some(crossing_point)
                        } else {
                            last
                        }
                    } else {
                        Some(crossing_point)
                    }
                } else {
                    last
                };
                match (start, end) {
                    (Some(start), Some(end)) => alloc(connection, start, end),
                    (Some(point), None) | (None, Some(point)) => alloc(connection, point, point),
                    (None, None) => panic!("Must contain at least one parent"),
                }
            };
            if matches!(first, Some(index) if index == item_index) {
                let origin = if last.is_some() {
                    Origin::Right
                } else {
                    Origin::None
                };
                connections.ending.push(Ending {
                    connection,
                    color: color(connection),
                    origin,
                    x_index: ppp(&mut connections.total_x),
                    y_index,
                });
            } else if matches!(last, Some(index) if index == item_index) {
                let origin = if first.is_some() {
                    Origin::Left
                } else {
                    Origin::None
                };
                connections.ending.push(Ending {
                    connection,
                    color: color(connection),
                    origin,
                    x_index: ppp(&mut connections.total_x),
                    y_index,
                });
            } else if matches!((first, last),
                                (Some(start), Some(end)) if start < item_index && item_index < end)
            {
                connections.passing.push(Passing {
                    connection,
                    color: color(connection),
                    y_index,
                })
            }
            if let Some(crossing_point) = rel_indices.crossing_point {
                let x_index = if let Some(ending) = connections.ending.iter().find(|ending| {
                    ending.connection == connection
                        && matches!((first, last), (Some(_), None) | (None, Some(_)))
                }) {
                    // if it's a single parent, connect crossing directly to ending
                    ending.x_index
                } else {
                    ppp(&mut connections.total_x)
                };
                if item_index == crossing_point {
                    connections.crossing.push(Crossing {
                        connection,
                        color: color(connection),
                        origin: Origin::None,
                        x_index,
                        y_index,
                    });
                }
            }
        });
    GridItem::Connections(connections)
}

fn middle(a: usize, b: usize) -> usize {
    let diff = if a <= b { b - a } else { a - b };
    if diff % 2 == 0 {
        diff / 2
    } else {
        (diff - 1) / 2
    }
}

/// Postfix plus plus
/// E.g. i++
fn ppp(i: &mut u32) -> u32 {
    let result = *i;
    *i += 1;
    result
}

fn color(connection: u32) -> Color {
    let fraction = (connection % 6) as f32 / 6f32;
    ((360.0 * fraction), 70.0, 50.0)
}
