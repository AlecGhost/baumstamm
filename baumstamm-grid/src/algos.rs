use crate::{Grid, indices::PersonIndex};
use baumstamm_lib::{PersonId, Relationship, RelationshipId};
use serde::{Deserialize, Serialize};
use specta::Type;

mod centered;
mod force_directed;
mod lexicographic;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub enum LayoutAlgorithm {
    Centered,
    #[default]
    ForceDirected,
    Lexicographic,
}

pub(super) fn get_person_indices(
    person_layers: &Grid<PersonId>,
    relationship_layers: &Grid<RelationshipId>,
    relationships: &[Relationship],
    row_length: usize,
    layout_algorithm: LayoutAlgorithm,
) -> Grid<PersonIndex> {
    match layout_algorithm {
        LayoutAlgorithm::Centered => centered::get_person_indices(person_layers, row_length),
        LayoutAlgorithm::ForceDirected => force_directed::get_person_indices(
            person_layers,
            relationship_layers,
            relationships,
            row_length,
        ),
        LayoutAlgorithm::Lexicographic => lexicographic::get_person_indices(
            person_layers,
            relationship_layers,
            relationships,
            row_length,
        ),
    }
}
