use baumstamm_lib::{FamilyTree, graph::Graph};
use indices::{PersonIndex, RelIndices};
use itertools::Itertools;

pub use algos::LayoutAlgorithm;
pub use items::GridItem;

use crate::items::Orientation;

mod algos;
mod indices;
mod items;
mod lines;

type Grid<T> = Vec<Vec<T>>;

pub fn generate(tree: &FamilyTree) -> Grid<GridItem> {
    generate_with_layout(tree, LayoutAlgorithm::default())
}

pub fn generate_with_layout(
    tree: &FamilyTree,
    layout_algorithm: LayoutAlgorithm,
) -> Grid<GridItem> {
    let rels = tree.get_relationships();
    let graph = Graph::new(rels).cut();
    let layers = graph.layers();
    let person_layers = graph.person_layers(rels);
    let widest_generation = person_layers
        .iter()
        .map(|layer| layer.len())
        .max()
        .unwrap_or_default();
    if widest_generation == 0 {
        return Vec::new();
    }
    let output = algos::get_person_indices(algos::PersonIndexInput {
        person_layers: &person_layers,
        relationship_layers: &layers,
        relationships: rels,
        layout_algorithm,
    });
    let rel_indices = indices::get_rel_indices(&layers, rels, &output.person_indices);

    fill_grid(&output.person_indices, &rel_indices, output.row_length)
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
    use baumstamm_lib::{
        PersonId,
        view::{View, ViewLimit, ViewOptions},
    };

    #[test]
    fn relationship_forces_are_the_default_layout() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");

        assert_eq!(LayoutAlgorithm::default(), LayoutAlgorithm::ForceDirected);
        assert_eq!(
            format!("{:?}", generate(&tree)),
            format!(
                "{:?}",
                generate_with_layout(&tree, LayoutAlgorithm::ForceDirected)
            )
        );
    }

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

    #[test]
    fn connection_optimized_layout_preserves_real_tree_generations() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");
        let centered = generate_with_layout(&tree, LayoutAlgorithm::Centered);
        let optimized = generate_with_layout(&tree, LayoutAlgorithm::ConnectionOptimized);

        let people_by_row = |grid: &Grid<GridItem>| {
            grid.iter()
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
        };

        assert_eq!(people_by_row(&centered), people_by_row(&optimized));
        assert!(optimized.iter().all(|row| row.len() == optimized[0].len()));
    }

    #[test]
    fn got_aenys_descendant_view_generates_a_grid() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");
        let root =
            PersonId::try_from("D22A2ABE9989009EFF43E7FD01BD033B").expect("valid Aenys I id");
        let options = ViewOptions {
            ancestor_gen_limit: ViewLimit::Limit(0),
            ..ViewOptions::default()
        };
        let view = View::new(&tree, root, &options).expect("valid descendant view");
        let view_tree = FamilyTree::from(view);

        let grid = generate(&view_tree);
        let grid_person_count = grid
            .iter()
            .flatten()
            .filter_map(|item| match item {
                GridItem::Person(person) => Some(person),
                GridItem::Connections(_) => None,
            })
            .unique()
            .count();

        assert_eq!(grid_person_count, view_tree.get_persons().len());
    }

    #[test]
    fn kinship_expansion_computes_enough_width_for_the_whole_tree() {
        let tree = FamilyTree::try_from(include_str!("../../examples/lotr/lotr.json"))
            .expect("valid example tree");
        let grid = generate_with_layout(&tree, LayoutAlgorithm::KinshipExpansion);

        assert!(!grid.is_empty());
        assert!(grid.iter().all(|row| row.len() == grid[0].len()));
        assert!(grid[0].len() >= tree.get_persons().len());
    }

    #[test]
    fn kinship_expansion_is_deterministic_and_preserves_occurrences() {
        let tree = FamilyTree::try_from(include_str!("../../examples/lotr/lotr.json"))
            .expect("valid example tree");
        let centered = generate_with_layout(&tree, LayoutAlgorithm::Centered);
        let expansion = generate_with_layout(&tree, LayoutAlgorithm::KinshipExpansion);

        let people_by_row = |grid: &Grid<GridItem>| {
            grid.iter()
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
        };

        assert_eq!(people_by_row(&centered), people_by_row(&expansion));
        assert_eq!(
            format!("{expansion:?}"),
            format!(
                "{:?}",
                generate_with_layout(&tree, LayoutAlgorithm::KinshipExpansion)
            )
        );
    }

    #[test]
    fn kinship_expansion_large_layout_is_collision_free_and_complete() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");
        let grid = generate_with_layout(&tree, LayoutAlgorithm::KinshipExpansion);
        let displayed_people = grid
            .iter()
            .flatten()
            .filter_map(|item| match item {
                GridItem::Person(person) => Some(*person),
                GridItem::Connections(_) => None,
            })
            .unique()
            .count();

        assert!(grid.iter().all(|row| row.len() == grid[0].len()));
        for row in grid.iter().skip(2).step_by(3) {
            assert_eq!(
                row.iter()
                    .filter_map(|item| match item {
                        GridItem::Person(person) => Some(*person),
                        GridItem::Connections(_) => None,
                    })
                    .unique()
                    .count(),
                row.iter()
                    .filter(|item| matches!(item, GridItem::Person(_)))
                    .count()
            );
        }
        assert_eq!(displayed_people, tree.get_persons().len());
    }
}
