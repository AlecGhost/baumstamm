use crate::{indices::RelIndices, items::Fraction};
use itertools::Itertools;

type Rid = baumstamm_lib::RelationshipId;

#[derive(Clone, Debug)]
pub struct HLine {
    pub rid: Rid,
    pub start: usize,
    pub end: usize,
}

pub struct VLine {
    pub rid: Rid,
    pub top: bool,
    pub middle: bool,
    pub bottom: bool,
}

pub struct AllocatedLine<Line> {
    pub pos: Fraction,
    pub line: Line,
}

pub fn create_horizontal(row: &[RelIndices]) -> [Vec<HLine>; 2] {
    let lines = row
        .iter()
        .map(|ri| {
            let parent_line = create_parent_line(ri);
            let children_line = create_children_line(ri);
            (parent_line, children_line)
        })
        .collect_vec();
    let (parent_lines, children_lines): (Vec<Option<HLine>>, Vec<Option<HLine>>) =
        lines.into_iter().unzip();
    let parent_lines = parent_lines.into_iter().flatten().collect_vec();
    let children_lines = children_lines.into_iter().flatten().collect_vec();
    [parent_lines, children_lines]
}

fn create_parent_line(ri: &RelIndices) -> Option<HLine> {
    match (ri.parents, ri.crossing_point) {
        ([Some(first), Some(second)], Some(crossing)) => {
            let points = [first, second, crossing];
            Some(HLine {
                rid: ri.rid,
                start: *points.iter().min().unwrap(),
                end: *points.iter().max().unwrap(),
            })
        }
        ([Some(parent), None], Some(crossing)) | ([None, Some(parent)], Some(crossing))
            if parent != crossing =>
        {
            let points = [parent, crossing];
            Some(HLine {
                rid: ri.rid,
                start: *points.iter().min().unwrap(),
                end: *points.iter().max().unwrap(),
            })
        }
        ([Some(first), Some(second)], None) => {
            let points = [first, second];
            Some(HLine {
                rid: ri.rid,
                start: *points.iter().min().unwrap(),
                end: *points.iter().max().unwrap(),
            })
        }
        _ => None,
    }
}

fn create_children_line(ri: &RelIndices) -> Option<HLine> {
    match (
        ri.children.first(),
        ri.children.last(),
        ri.crossing_point.as_ref(),
    ) {
        (Some(first), Some(second), Some(crossing)) => {
            let points = [first, second, crossing];
            let start = **points.iter().min().unwrap();
            let end = **points.iter().max().unwrap();
            if start == end {
                return None;
            }
            Some(HLine {
                rid: ri.rid,
                start,
                end,
            })
        }
        (Some(first), Some(second), None) => {
            let points = [first, second];
            let start = **points.iter().min().unwrap();
            let end = **points.iter().max().unwrap();
            if start == end {
                return None;
            }
            Some(HLine {
                rid: ri.rid,
                start,
                end,
            })
        }
        _ => None,
    }
}

pub fn allocate_horizontal(lines: Vec<HLine>) -> Vec<AllocatedLine<HLine>> {
    fn lines_at_index(lines: &[HLine], index: usize) -> Vec<&HLine> {
        lines
            .iter()
            .filter(|line| line.start == index)
            .collect_vec()
    }

    if lines.is_empty() {
        return Vec::new();
    }
    let min = lines
        .iter()
        .map(|line| {
            assert!(line.start < line.end, "line layout");
            line.start
        })
        .min()
        .expect("Lines is not empty");
    let max = lines
        .iter()
        .map(|line| line.end)
        .max()
        .expect("Lines is not empty");
    assert!(min < max, "Min must be smaller than max");
    let mut allocated_lines: Vec<(usize, Rid)> = Vec::new();
    let mut allocator: Vec<usize> = Vec::new();
    (min..max).for_each(|index| {
        let lines_to_allocate = lines_at_index(&lines, index);
        lines_to_allocate.into_iter().for_each(|line| {
            if let Some((space, counter)) = allocator
                .iter_mut()
                .find_position(|counter| **counter <= index)
            {
                *counter = line.end + 1;
                allocated_lines.push((space, line.rid));
            } else {
                allocator.push(line.end + 1);
                let space = allocator.len() - 1;
                allocated_lines.push((space, line.rid));
            }
        });
    });
    let denominator = allocator.len() + 1;
    lines
        .into_iter()
        .map(|line| {
            let numerator = allocated_lines
                .iter()
                .find(|(_, rid)| line.rid == *rid)
                .map(|(space, _)| space)
                .cloned()
                .map(|space| space + 1)
                .expect("All lines must be allocated");
            let pos = Fraction {
                numerator,
                denominator,
            };
            AllocatedLine { pos, line }
        })
        .collect_vec()
}

pub fn allocate_vertical(
    rel_indices: &[RelIndices],
    row_length: usize,
) -> Vec<Vec<AllocatedLine<VLine>>> {
    (0..row_length)
        .map(|index| {
            let top_endings = rel_indices
                .iter()
                .filter(|ri| ri.get_parents().contains(&index))
                .map(|ri| ri.rid)
                .collect_vec();
            let middle_crossings = rel_indices
                .iter()
                .filter(|ri| matches!(ri.crossing_point, Some(crossing_point) if crossing_point == index))
                .map(|ri| ri.rid)
                .collect_vec();
            let bottom_endings = rel_indices
                .iter()
                .filter(|ri| ri.children.contains(&index))
                .filter(|ri| ri.children.len() > 1 || ri.get_parents().len() != 0)
                .map(|ri| ri.rid)
                .collect_vec();
            let lines = rel_indices.iter().map(|ri| ri.rid).flat_map(|rid| {
                let top = top_endings.contains(&rid);
                let middle = middle_crossings.contains(&rid);
                let bottom = bottom_endings.contains(&rid);
                match (top, middle, bottom) {
                    (_, true, _) => {
                        // crosses the middle, endings must be connected to the crossing line
                        vec![VLine {
                            rid,
                            top,
                            middle,
                            bottom,
                        }]
                    },
                    (true, false, true) => {
                        // does not cross, so top and bottom are independent
                        vec![
                            VLine {
                                rid,
                                top: true,
                                middle: false,
                                bottom: false,
                            },
                            VLine {
                                rid,
                                top: false,
                                middle: false,
                                bottom: true,
                            },
                        ]
                    },
                    (true, false, false) | (false, false, true) => {
                        // does not cross, only top or bottom exists, so creating just one line is safe
                        vec![VLine {
                            rid,
                            top,
                            middle,
                            bottom,
                        }]
                    }
                    (false, false, false) => {
                        // no line to draw
                        Vec::new()
                    }
                }
            }).collect_vec();

            enum Occupation {
                Upper,
                Lower,
                Both,
            }

            impl VLine {
                fn occupation(&self) -> Occupation {
                    match (self.top, self.middle, self.bottom) {
                        (_, true, _) => Occupation::Both,
                        (true, false, false) =>Occupation::Upper,
                        (false, false, true) => Occupation::Lower,
                        (true, false, true)=>panic!("VLine must not be top and bottom."),
                        (false, false, false)=>panic!("VLine must not have no parts."),
                    }
                }
            }

            let mut allocator: Vec<Occupation> = Vec::new();
            fn allocate(allocator: &mut Vec<Occupation>, line: &VLine) -> usize {
                match line.occupation() {
                    Occupation::Both => {
                        allocator.push(Occupation::Both);
                        allocator.len() - 1
                    }
                    Occupation::Upper => {
                        if let Some((index, occupation)) = allocator.iter_mut()
                            .find_position(|occupation| matches!(occupation, Occupation::Lower)) {
                            *occupation = Occupation::Both;
                            index
                        } else {
                            allocator.push(Occupation::Upper);
                            allocator.len() - 1
                        }
                    }
                    Occupation::Lower => {
                        if let Some((index, occupation)) = allocator.iter_mut()
                            .find_position(|occupation| matches!(occupation, Occupation::Upper)) {
                            *occupation = Occupation::Both;
                            index
                        } else {
                            allocator.push(Occupation::Lower);
                            allocator.len() - 1
                        }
                    }
                }
            }
            let allocated_lines = lines.into_iter().map(|line| {
                (allocate(&mut allocator, &line), line)
            }).collect_vec();
            let denominator = allocator.len() + 1;
            allocated_lines.into_iter()
                .map(|(index, line)| (index + 1, line))
                .map(|(numerator, line)| {
                AllocatedLine {
                    pos: Fraction { numerator, denominator },
                    line
                }
            }).collect_vec()
        })
        .collect_vec()
}
