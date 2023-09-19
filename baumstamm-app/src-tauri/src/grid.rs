use baumstamm_lib::{graph::Graph, FamilyTree};
use indices::{PersonIndex, RelIndices};
use items::Connections;
use itertools::Itertools;
use std::collections::HashMap;

pub use items::GridItem;

mod indices;
mod items;

type Grid<T> = Vec<Vec<T>>;

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

    fn get_allocated(&self) -> &HashMap<u32, usize> {
        &self.allocated
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
    let person_indices = indices::get_person_indices(&person_layers, row_length, rels);
    let rel_indices = indices::get_rel_indices(&layers, rels, &person_indices);

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
        let mut line_allocator = LineAllocator::default();
        let mut alloc = |connection, start, end| line_allocator.alloc(connection, start, end);
        let mut layer = match layer_index % 3 {
            0 => {
                // sibling layer
                let rel_indices = rel_indices[layer_index / 3].as_slice();
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
                        alloc(connection, start, end);
                    });
                row(row_length, |item_index| {
                    items::new_sibling_item(
                        rel_indices,
                        item_index,
                        get_rel_connections(&mut grid, layer_index, item_index),
                        line_allocator.get_allocated(),
                    )
                })
            }
            1 => {
                // person layer
                let person_indices = person_indices[(layer_index - 1) / 3].as_slice();
                row(row_length, |item_index| {
                    items::new_person_item(person_indices, item_index)
                })
            }
            2 => {
                // relationship layer
                let rel_indices = rel_indices[(layer_index - 2) / 3 + 1].as_slice();
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
                        let start = match (rel_indices.crossing_point, first) {
                            (Some(crossing_point), Some(first)) if crossing_point < first => {
                                Some(crossing_point)
                            }
                            (Some(crossing_point), None) => Some(crossing_point),
                            _ => first,
                        };
                        let end = last.max(rel_indices.crossing_point);
                        match (start, end) {
                            (Some(start), Some(end)) => alloc(connection, start, end),
                            (Some(point), None) | (None, Some(point)) => {
                                alloc(connection, point, point)
                            }
                            (None, None) => panic!("Must contain at least one parent"),
                        };
                    });
                row(row_length, |item_index| {
                    items::new_relationship_item(
                        rel_indices,
                        item_index,
                        line_allocator.get_allocated(),
                    )
                })
            }
            _ => panic!("Math broken"),
        };
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

fn row<F: FnMut(usize) -> GridItem>(row_length: usize, item_fn: F) -> Vec<GridItem> {
    (0..row_length).map(item_fn).collect_vec()
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
