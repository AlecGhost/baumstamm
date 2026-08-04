use crate::{
    Grid,
    indices::{PersonIndex, middle},
};
use baumstamm_lib::PersonId;
use itertools::Itertools;

pub(super) fn get_person_indices(
    person_layers: &Grid<PersonId>,
    row_length: usize,
) -> Grid<PersonIndex> {
    person_layers
        .iter()
        .map(|layer| {
            let start_index = middle(layer.len(), row_length);
            layer
                .iter()
                .enumerate()
                .map(|(i, pid)| PersonIndex {
                    index: start_index + i,
                    pid: *pid,
                })
                .collect_vec()
        })
        .collect_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(value: u128) -> PersonId {
        value.into()
    }

    fn slots(indices: &[PersonIndex]) -> Vec<(u128, usize)> {
        let mut slots = indices
            .iter()
            .map(|person| (person.pid.0, person.index))
            .collect_vec();
        slots.sort_by_key(|(_, index)| *index);
        slots
    }

    #[test]
    fn centered_layout_remains_available() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(3)]];
        let indices = get_person_indices(&layers, 2);

        assert_eq!(slots(&indices[0]), vec![(1, 0), (2, 1)]);
        assert_eq!(slots(&indices[1]), vec![(3, 0)]);
    }
}
