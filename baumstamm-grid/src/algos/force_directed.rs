use super::PersonIndexInput;
use crate::{Grid, indices::PersonIndex};

type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> Grid<PersonIndex> {
    todo!()
}
