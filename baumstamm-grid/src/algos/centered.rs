use super::{PersonIndexInput, PersonIndexOutput};
use crate::indices::{PersonIndex, middle};
use itertools::Itertools;

pub fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    let row_length = input
        .person_layers
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or_default();
    let person_indices = input
        .person_layers
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
        .collect_vec();
    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use baumstamm_lib::PersonId;

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
        let output = get_person_indices(PersonIndexInput {
            person_layers: &layers,
            relationship_layers: &vec![],
            relationships: &[],
            layout_algorithm: LayoutAlgorithm::Centered,
        });

        assert_eq!(output.row_length, 2);
        assert_eq!(slots(&output.person_indices[0]), vec![(1, 0), (2, 1)]);
        assert_eq!(slots(&output.person_indices[1]), vec![(3, 0)]);
    }
}
