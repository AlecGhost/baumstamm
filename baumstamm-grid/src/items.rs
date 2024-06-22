use crate::lines::{AllocatedLine, HLine, VLine};
use itertools::Itertools;

use super::indices::PersonIndex;
use serde::{Deserialize, Serialize};
use specta::Type;

type Color = (f32, f32, f32);
type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

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
    passing: Vec<Passing>,
    ending: Vec<Ending>,
    crossing: Vec<Crossing>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, Type)]
pub enum Orientation {
    #[default]
    /// The row is oriented upward, i.e. towards the parents
    Up,
    /// The row is oriented downward, i.e. towards the children
    Down,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Passing {
    rid: Rid,
    color: Color,
    y_fraction: Fraction,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Ending {
    rid: Rid,
    color: Color,
    origin: Origin,
    x_fraction: Fraction,
    y_fraction: Fraction,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Crossing {
    rid: Rid,
    color: Color,
    origin: Origin,
    x_fraction: Fraction,
    y_fraction: Fraction,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub enum Origin {
    /// The line originates from the left side
    Left,
    /// The line originates from the right side
    Right,
    /// The line does not have an origin
    None,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct Fraction {
    pub numerator: usize,
    pub denominator: usize,
}

pub fn new_person_row(person_indices: &[PersonIndex], row_length: usize) -> Vec<GridItem> {
    (0..row_length)
        .map(|item_index| {
            person_indices
                .iter()
                .find(|pi| pi.index == item_index)
                .map(|pi| GridItem::Person(pi.pid))
                .unwrap_or_default()
        })
        .collect_vec()
}

pub fn new_connection_row(
    vertical_lines: &[Vec<AllocatedLine<VLine>>],
    horizontal_lines: &[AllocatedLine<HLine>],
    orientation: Orientation,
) -> Vec<GridItem> {
    vertical_lines
        .iter()
        .enumerate()
        .map(|(index, vlines)| {
            let passing = horizontal_lines
                .iter()
                .filter(|allocated_line| {
                    allocated_line.line.start < index && index < allocated_line.line.end
                })
                .map(|allocated_line| Passing {
                    color: color(allocated_line.line.rid),
                    rid: allocated_line.line.rid,
                    y_fraction: allocated_line.pos.clone(),
                })
                .collect_vec();
            let crossing = vlines
                .iter()
                .filter(|vline| vline.line.middle)
                .map(|vline| {
                    let rid = vline.line.rid;
                    let x_fraction = vline.pos.clone();
                    if let Some(hline) = horizontal_lines.iter().find(|line| line.line.rid == rid) {
                        let y_fraction = hline.pos.clone();
                        let origin = if index <= hline.line.start {
                            Origin::Right
                        } else if hline.line.end <= index {
                            Origin::Left
                        } else {
                            Origin::None
                        };
                        Crossing {
                            rid,
                            color: color(rid),
                            origin,
                            x_fraction,
                            y_fraction,
                        }
                    } else {
                        let y_fraction = Fraction {
                            numerator: 0,
                            denominator: 1,
                        };
                        let origin = Origin::None;
                        Crossing {
                            rid,
                            color: color(rid),
                            origin,
                            x_fraction,
                            y_fraction,
                        }
                    }
                })
                .collect_vec();
            let ending = vlines
                .iter()
                .filter(|vline| {
                    (matches!(orientation, Orientation::Up) && vline.line.top)
                        || (matches!(orientation, Orientation::Down) && vline.line.bottom)
                })
                .map(|vline| {
                    let rid = vline.line.rid;
                    let x_fraction = vline.pos.clone();
                    if let Some(hline) = horizontal_lines.iter().find(|line| line.line.rid == rid) {
                        let y_fraction = hline.pos.clone();
                        let origin = if index <= hline.line.start {
                            Origin::Right
                        } else if hline.line.end <= index {
                            Origin::Left
                        } else {
                            Origin::None
                        };
                        Ending {
                            rid,
                            color: color(rid),
                            origin,
                            x_fraction,
                            y_fraction,
                        }
                    } else {
                        let y_fraction = Fraction {
                            numerator: 0,
                            denominator: 1,
                        };
                        let origin = Origin::None;
                        Ending {
                            rid,
                            color: color(rid),
                            origin,
                            x_fraction,
                            y_fraction,
                        }
                    }
                })
                .collect_vec();
            GridItem::Connections(Connections {
                orientation: orientation.clone(),
                passing,
                ending,
                crossing,
            })
        })
        .collect_vec()
}

fn color(rid: Rid) -> Color {
    let fraction = (rid.0 % 6) as f32 / 6f32;
    ((360.0 * fraction), 70.0, 50.0)
}
