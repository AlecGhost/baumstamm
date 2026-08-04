use crate::{Grid, indices::PersonIndex};
use baumstamm_lib::{PersonId, Relationship, RelationshipId};
use serde::{Deserialize, Serialize};
use specta::Type;

mod centered;
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
}
