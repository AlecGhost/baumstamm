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

#[derive(Clone, Copy)]
pub(super) struct PersonIndexInput<'a> {
    pub person_layers: &'a Grid<PersonId>,
    pub relationship_layers: &'a Grid<RelationshipId>,
    pub relationships: &'a [Relationship],
    pub row_length: usize,
    pub layout_algorithm: LayoutAlgorithm,
}

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> Grid<PersonIndex> {
    match input.layout_algorithm {
        LayoutAlgorithm::Centered => centered::get_person_indices(input),
        LayoutAlgorithm::ForceDirected => force_directed::get_person_indices(input),
        LayoutAlgorithm::Lexicographic => lexicographic::get_person_indices(input),
    }
}
