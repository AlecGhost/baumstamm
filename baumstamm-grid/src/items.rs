use std::collections::HashMap;

use super::indices::{PersonIndex, RelIndices};
use serde::{Deserialize, Serialize};
use specta::Type;

type Color = (f32, f32, f32);
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
    total_x: u32,
    pub total_y: u32,
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

pub fn new_sibling_item(
    rel_indices: &[RelIndices],
    item_index: usize,
    mut rel_connections: Option<&mut Connections>,
    y_indices: &HashMap<u32, usize>,
) -> GridItem
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
            let y_index = y_indices
                .get(&connection)
                .cloned()
                .expect("Must be allocated")
                .try_into()
                .expect("Too many relationships");
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

pub fn new_person_item(person_indices: &[PersonIndex], item_index: usize) -> GridItem {
    person_indices
        .iter()
        .find(|pi| pi.index == item_index)
        .map(|pi| GridItem::Person(pi.pid))
        .unwrap_or_default()
}

pub fn new_relationship_item(
    rel_indices: &[RelIndices],
    item_index: usize,
    y_indices: &HashMap<u32, usize>,
) -> GridItem {
    let mut connections = Connections {
        orientation: Orientation::Up,
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
            let connection = connection.try_into().expect("Too many relationships");
            let y_index = y_indices
                .get(&connection)
                .cloned()
                .expect("Must be allocated")
                .try_into()
                .expect("Too many relationships");
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
