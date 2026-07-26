use baumstamm_lib::{graph::Graph, FamilyTree};
use indices::{PersonIndex, RelIndices};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;

pub use items::GridItem;

use crate::items::Orientation;

mod indices;
mod items;
mod lines;

type Grid<T> = Vec<Vec<T>>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum LayoutAlgorithm {
    #[default]
    Centered,
    ForceDirected,
}

pub fn generate(tree: &FamilyTree) -> Grid<GridItem> {
    generate_with_layout(tree, LayoutAlgorithm::Centered)
}

pub fn generate_with_layout(
    tree: &FamilyTree,
    layout_algorithm: LayoutAlgorithm,
) -> Grid<GridItem> {
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
    let person_indices =
        indices::get_person_indices(&person_layers, rels, row_length, layout_algorithm);
    let rel_indices = indices::get_rel_indices(&layers, rels, &person_indices);

    fill_grid(&person_indices, &rel_indices, row_length)
}

/// Fill grid with `GridItem`s
fn fill_grid(
    person_indices: &Grid<PersonIndex>,
    rel_indices: &Grid<RelIndices>,
    row_length: usize,
) -> Grid<GridItem> {
    // persons
    let mut person_rows = person_indices
        .iter()
        .map(|pi_row| items::new_person_row(pi_row, row_length))
        .collect_vec();

    // horizontal lines
    let horizontal_lines = rel_indices
        .iter()
        .flat_map(|row| lines::create_horizontal(row))
        .collect_vec();
    let allocated_horizontal_lines = horizontal_lines
        .into_iter()
        .map(lines::allocate_horizontal)
        .collect_vec();
    assert_eq!(
        allocated_horizontal_lines.len() % 2,
        0,
        "Always one parent and one children line"
    );

    // vertical lines
    let allocated_vertical_lines = rel_indices
        .iter()
        .map(|row| lines::allocate_vertical(row, row_length))
        .collect_vec();
    for row in allocated_vertical_lines.iter() {
        assert_eq!(row.len(), row_length, "All rows must match the grid.");
    }

    assert_eq!(
        allocated_horizontal_lines.len(),
        allocated_vertical_lines.len() * 2,
        "Twice as many horizontal lines as vertical lines."
    );

    // connections
    let mut connection_rows = allocated_vertical_lines
        .iter()
        // duplicate vertical lines to line up with horizontal lines
        .flat_map(|row| [(row, Orientation::Up), (row, Orientation::Down)])
        .zip(&allocated_horizontal_lines)
        .map(|((vertical, orientation), horizontal)| {
            items::new_connection_row(vertical, horizontal, orientation)
        })
        .collect_vec();

    // combine person and connection rows to build grid
    let nr_of_layers = person_rows.len() * 3;
    let mut grid = Vec::with_capacity(nr_of_layers);
    // TODO: why do we need another row?
    if connection_rows.len() > person_rows.len() * 2 {
        person_rows.push((0..row_length).map(|_| GridItem::default()).collect_vec());
    }
    person_rows.reverse();
    connection_rows.reverse();

    assert_eq!(
        connection_rows.len(),
        person_rows.len() * 2,
        "Twice as many person rows as connection rows."
    );

    for i in 0..nr_of_layers {
        if i % 3 == 2 {
            grid.push(person_rows.pop().expect("Person row is missing"));
        } else {
            grid.push(connection_rows.pop().expect("Connection row is missing"));
        }
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_layout_preserves_real_tree_generations_and_is_deterministic() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");
        let centered = generate_with_layout(&tree, LayoutAlgorithm::Centered);
        let force = generate_with_layout(&tree, LayoutAlgorithm::ForceDirected);

        assert_eq!(centered.len(), force.len());
        assert_eq!(
            centered
                .iter()
                .map(|row| {
                    row.iter()
                        .filter_map(|item| match item {
                            GridItem::Person(person) => Some(*person),
                            GridItem::Connections(_) => None,
                        })
                        .sorted()
                        .collect_vec()
                })
                .collect_vec(),
            force
                .iter()
                .map(|row| {
                    row.iter()
                        .filter_map(|item| match item {
                            GridItem::Person(person) => Some(*person),
                            GridItem::Connections(_) => None,
                        })
                        .sorted()
                        .collect_vec()
                })
                .collect_vec()
        );
        assert!(force.iter().all(|row| row.len() == force[0].len()));
        assert_eq!(
            format!("{force:?}"),
            format!(
                "{:?}",
                generate_with_layout(&tree, LayoutAlgorithm::ForceDirected)
            )
        );
    }
}
