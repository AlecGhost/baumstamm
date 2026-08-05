use crate::{Grid, indices::PersonIndex};
use baumstamm_lib::{PersonId, Relationship, RelationshipId};
use serde::{Deserialize, Serialize};
use specta::Type;

mod centered;
mod common;
mod connection_optimized;
mod force_directed;
mod kinship_expansion;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum LayoutAlgorithm {
    Centered,
    /// Minimizes rendered horizontal-segment congestion, then total span.
    ConnectionOptimized,
    #[default]
    ForceDirected,
    #[serde(alias = "Lexicographic")]
    KinshipExpansion,
}

#[derive(Clone, Copy)]
pub struct PersonIndexInput<'a> {
    pub person_layers: &'a Grid<PersonId>,
    pub relationship_layers: &'a Grid<RelationshipId>,
    pub relationships: &'a [Relationship],
    pub layout_algorithm: LayoutAlgorithm,
}

pub struct PersonIndexOutput {
    pub person_indices: Grid<PersonIndex>,
    pub row_length: usize,
}

pub fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    match input.layout_algorithm {
        LayoutAlgorithm::Centered => centered::get_person_indices(input),
        LayoutAlgorithm::ConnectionOptimized => connection_optimized::get_person_indices(input),
        LayoutAlgorithm::ForceDirected => force_directed::get_person_indices(input),
        LayoutAlgorithm::KinshipExpansion => kinship_expansion::get_person_indices(input),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baumstamm_lib::{FamilyTree, graph::Graph};
    use serde::Deserialize;

    #[test]
    fn legacy_lexicographic_name_deserializes_as_kinship_expansion() {
        let deserializer =
            serde::de::value::StrDeserializer::<serde::de::value::Error>::new("Lexicographic");

        assert_eq!(
            LayoutAlgorithm::deserialize(deserializer).expect("legacy layout name"),
            LayoutAlgorithm::KinshipExpansion
        );
    }

    #[test]
    fn got_layouts_are_compact_and_relationship_aware() {
        let tree = FamilyTree::try_from(include_str!("../../examples/got/got.json"))
            .expect("valid example tree");
        let relationships = tree.get_relationships();
        let graph = Graph::new(relationships).cut();
        let relationship_layers = graph.layers();
        let person_layers = graph.person_layers(relationships);

        let metrics = |algorithm| {
            let output = get_person_indices(PersonIndexInput {
                person_layers: &person_layers,
                relationship_layers: &relationship_layers,
                relationships,
                layout_algorithm: algorithm,
            });
            let relationship_indices = crate::indices::get_rel_indices(
                &relationship_layers,
                relationships,
                &output.person_indices,
            );
            let mut congestion = 0_u64;
            let mut length = 0_u64;
            let mut layer_lengths = Vec::new();
            for row in &relationship_indices {
                let mut layer_length = 0_u64;
                for channel in crate::lines::create_horizontal(row) {
                    let mut occupation = vec![0_u64; output.row_length];
                    for line in channel {
                        let line_length = line.end.abs_diff(line.start) as u64;
                        length += line_length;
                        layer_length += line_length;
                        for count in &mut occupation[line.start..=line.end] {
                            congestion += *count;
                            *count += 1;
                        }
                    }
                }
                layer_lengths.push(layer_length);
            }
            (output.row_length, congestion, length, layer_lengths)
        };

        let centered = metrics(LayoutAlgorithm::Centered);
        let force = metrics(LayoutAlgorithm::ForceDirected);
        let optimized = metrics(LayoutAlgorithm::ConnectionOptimized);
        let expansion = metrics(LayoutAlgorithm::KinshipExpansion);
        let widest_layer = person_layers.iter().map(Vec::len).max().unwrap();
        let youngest_connection_layer = person_layers.len() - 1;

        assert_eq!(force.0, widest_layer * 3 / 2);
        assert!(force.2 * 4 < centered.2 * 3);
        assert!(force.3[youngest_connection_layer] < centered.3[youngest_connection_layer]);

        assert_eq!(optimized.0, widest_layer);
        assert!(optimized.1 < force.1);
        assert!(optimized.2 < force.2);
        assert!(optimized.3[youngest_connection_layer] < force.3[youngest_connection_layer]);

        assert_eq!(expansion.0, widest_layer + widest_layer.div_ceil(8));
        assert!(expansion.1 < centered.1);
        assert!(expansion.2 < centered.2);
    }
}
