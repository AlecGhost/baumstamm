use baumstamm_lib::{graph::Graph, FamilyTree};
use indices::{PersonIndex, RelIndices};
use items::Connections;
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
    let person_indices = indices::get_person_indices(&person_layers, row_length);
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
        let mut layer = Vec::new();
        let mut line_allocator = LineAllocator::default();
        for item_index in 0..row_length {
            let item = match layer_index % 3 {
                0 => items::new_sibling_item(
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
                1 => items::new_person_item(
                    person_indices[(layer_index - 1) / 3].as_slice(),
                    item_index,
                ),
                2 => items::new_relationship_item(
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
