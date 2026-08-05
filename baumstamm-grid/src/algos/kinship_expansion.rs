use super::{PersonIndexInput, PersonIndexOutput, common::project_to_slots};
use crate::indices::PersonIndex;
use baumstamm_lib::PersonId;
use std::collections::HashSet;

type Pid = PersonId;

const EXTRA_WIDTH_DIVISOR: usize = 8;
const REFINEMENT_PASSES: usize = 4;
const CENTER_GRAVITY_WEIGHT: usize = 1;
const UNRELATED_TARGET_WEIGHT: usize = 1;
const REPEATED_PERSON_WEIGHT: usize = 10;
const PARENT_GROUP_WEIGHT: usize = 20;
const CHILD_GROUP_WEIGHT: usize = 8;
const SPOUSE_WEIGHT: usize = 12;
const SIBLING_SLOT_SPACING: f64 = 1.0;

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    let widest_layer = input
        .person_layers
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or_default();
    if widest_layer == 0 {
        return PersonIndexOutput {
            person_indices: input.person_layers.iter().map(|_| Vec::new()).collect(),
            row_length: 0,
        };
    }

    // A small, bounded amount of breathing room lets sibling groups and
    // spouses shift as a unit without making width proportional to the total
    // number of people in the tree.
    let row_length = widest_layer.saturating_add(widest_layer.div_ceil(EXTRA_WIDTH_DIVISOR));
    let mut person_indices = centered_indices(input.person_layers, row_length);
    center_oldest_root(&mut person_indices, row_length);

    // Grow outwards from the oldest layer. Each new layer is ordered and
    // projected around its already placed parents (or the nearest repeated
    // occurrence), so descendant subtrees continue to use the space below the
    // root instead of receiving globally unique columns.
    for layer in 1..person_indices.len() {
        relayout_layer(&input, &mut person_indices, layer, row_length);
    }

    // A few alternating sweeps pull parents toward child groups and then
    // propagate those compact positions back down. The oldest layer stays
    // pinned, keeping the selected root centered.
    for _ in 0..REFINEMENT_PASSES {
        for layer in (1..person_indices.len()).rev() {
            relayout_layer(&input, &mut person_indices, layer, row_length);
        }
        for layer in 1..person_indices.len() {
            relayout_layer(&input, &mut person_indices, layer, row_length);
        }
    }

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

fn centered_indices(person_layers: &[Vec<Pid>], row_length: usize) -> Vec<Vec<PersonIndex>> {
    person_layers
        .iter()
        .map(|layer| {
            let start = (row_length - layer.len()) / 2;
            layer
                .iter()
                .enumerate()
                .map(|(offset, pid)| PersonIndex {
                    pid: *pid,
                    index: start + offset,
                })
                .collect()
        })
        .collect()
}

fn center_oldest_root(person_indices: &mut [Vec<PersonIndex>], row_length: usize) {
    let Some(layer) = person_indices.iter_mut().find(|layer| !layer.is_empty()) else {
        return;
    };
    let center = (row_length - 1) / 2;
    let mut slots = (0..row_length).collect::<Vec<_>>();
    slots.sort_by_key(|column| {
        (
            column.abs_diff(center),
            usize::from(*column < center),
            *column,
        )
    });
    for (person, column) in layer.iter_mut().zip(slots) {
        person.index = column;
    }
}

fn relayout_layer(
    input: &PersonIndexInput<'_>,
    person_indices: &mut [Vec<PersonIndex>],
    layer: usize,
    row_length: usize,
) {
    let center = (row_length - 1) as f64 / 2.0;
    let mut desired = person_indices[layer]
        .iter()
        .enumerate()
        .map(|(order, person)| {
            (
                person.pid,
                relative_target(input, person_indices, layer, person.pid)
                    .unwrap_or((center, UNRELATED_TARGET_WEIGHT)),
                order,
            )
        })
        .collect::<Vec<_>>();
    desired.sort_by(|first, second| {
        first
            .1
            .0
            .total_cmp(&second.1.0)
            .then_with(|| first.2.cmp(&second.2))
            .then_with(|| first.0.cmp(&second.0))
    });
    let targets = desired
        .iter()
        .map(|(_, (target, weight), _)| {
            // Weak gravity keeps disconnected source components in the middle
            // while close relatives dominate the placement.
            (target * *weight as f64 + center * CENTER_GRAVITY_WEIGHT as f64)
                / (*weight + CENTER_GRAVITY_WEIGHT) as f64
        })
        .collect::<Vec<_>>();
    let slots = project_to_slots(&targets, row_length);
    person_indices[layer] = desired
        .into_iter()
        .zip(slots)
        .map(|((pid, _, _), index)| PersonIndex { pid, index })
        .collect();

    debug_assert_eq!(
        person_indices[layer]
            .iter()
            .map(|person| person.index)
            .collect::<HashSet<_>>()
            .len(),
        person_indices[layer].len(),
        "people in one layer must occupy distinct columns"
    );
}

fn relative_target(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    layer: usize,
    pid: Pid,
) -> Option<(f64, usize)> {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0;
    let mut add = |column: f64, weight: usize| {
        weighted_sum += column * weight as f64;
        total_weight += weight;
    };

    for (other_layer, occurrences) in person_indices.iter().enumerate() {
        if other_layer != layer {
            for occurrence in occurrences.iter().filter(|person| person.pid == pid) {
                add(occurrence.index as f64, REPEATED_PERSON_WEIGHT);
            }
        }
    }

    for (relationship_layer, ids) in input.relationship_layers.iter().enumerate() {
        if relationship_layer != layer && relationship_layer != layer + 1 {
            continue;
        }
        for id in ids {
            let relationship = input
                .relationships
                .iter()
                .find(|relationship| relationship.id == *id)
                .expect("relationship layer references an unknown relationship");
            if relationship_layer == layer && relationship.children.contains(&pid) && layer > 0 {
                let columns = relationship
                    .parents
                    .iter()
                    .flatten()
                    .filter_map(|parent| {
                        person_indices[layer - 1]
                            .iter()
                            .find(|person| person.pid == *parent)
                            .map(|person| person.index)
                    })
                    .collect::<Vec<_>>();
                if let (Some(minimum), Some(maximum)) = (columns.iter().min(), columns.iter().max())
                {
                    let parent_center = (minimum + maximum) as f64 / 2.0;
                    let ordered_siblings =
                        ordered_siblings(input, person_indices, layer, &relationship.children);
                    let sibling_offset = ordered_siblings
                        .iter()
                        .position(|sibling| *sibling == pid)
                        .map(|position| position as f64 - (ordered_siblings.len() - 1) as f64 / 2.0)
                        .unwrap_or_default();
                    add(
                        parent_center + sibling_offset * SIBLING_SLOT_SPACING,
                        PARENT_GROUP_WEIGHT,
                    );
                }
            }
            if relationship_layer == layer + 1
                && relationship
                    .parents
                    .iter()
                    .flatten()
                    .any(|parent| *parent == pid)
            {
                if let Some(children) = person_indices.get(layer + 1) {
                    let columns = relationship
                        .children
                        .iter()
                        .filter_map(|child| {
                            children
                                .iter()
                                .find(|person| person.pid == *child)
                                .map(|person| person.index)
                        })
                        .collect::<Vec<_>>();
                    if let (Some(minimum), Some(maximum)) =
                        (columns.iter().min(), columns.iter().max())
                    {
                        add((minimum + maximum) as f64 / 2.0, CHILD_GROUP_WEIGHT);
                    }
                }
                for partner in relationship
                    .parents
                    .iter()
                    .flatten()
                    .filter(|partner| **partner != pid)
                {
                    if let Some(partner) = person_indices[layer]
                        .iter()
                        .find(|person| person.pid == *partner)
                    {
                        add(partner.index as f64, SPOUSE_WEIGHT);
                    }
                }
            }
        }
    }

    (total_weight > 0).then_some((weighted_sum / total_weight as f64, total_weight))
}

fn ordered_siblings(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    layer: usize,
    children: &[Pid],
) -> Vec<Pid> {
    let visible = children
        .iter()
        .copied()
        .filter(|child| {
            person_indices[layer]
                .iter()
                .any(|person| person.pid == *child)
        })
        .collect::<Vec<_>>();
    if visible.len() <= 2 {
        return visible;
    }

    let (partnered, unpartnered): (Vec<_>, Vec<_>) = visible
        .into_iter()
        .partition(|child| has_displayed_spouse(input, person_indices, layer, *child));
    let left_partner_count = partnered.len().div_ceil(2);
    partnered[..left_partner_count]
        .iter()
        .chain(&unpartnered)
        .chain(&partnered[left_partner_count..])
        .copied()
        .collect()
}

fn has_displayed_spouse(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    layer: usize,
    pid: Pid,
) -> bool {
    input
        .relationship_layers
        .get(layer + 1)
        .into_iter()
        .flatten()
        .filter_map(|id| {
            input
                .relationships
                .iter()
                .find(|relationship| relationship.id == *id)
        })
        .any(|relationship| {
            relationship
                .parents
                .iter()
                .flatten()
                .any(|parent| *parent == pid)
                && relationship.parents.iter().flatten().any(|partner| {
                    *partner != pid
                        && person_indices[layer]
                            .iter()
                            .any(|person| person.pid == *partner)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use baumstamm_lib::{Relationship, RelationshipId};

    fn pid(value: u128) -> Pid {
        value.into()
    }

    fn relationship(id: u128, members: &[u128]) -> Relationship {
        Relationship {
            id: RelationshipId(id),
            parents: [members.first().copied().map(pid), None],
            children: members.iter().skip(1).copied().map(pid).collect(),
        }
    }

    fn run(layers: &Vec<Vec<Pid>>, relationships: &[Relationship]) -> PersonIndexOutput {
        let mut relationship_layers = vec![Vec::new(); layers.len()];
        for relationship in relationships {
            for layer in 1..layers.len() {
                let parents_present = relationship
                    .parents
                    .iter()
                    .flatten()
                    .all(|parent| layers[layer - 1].contains(parent));
                let children_present = relationship
                    .children
                    .iter()
                    .all(|child| layers[layer].contains(child));
                if parents_present && children_present {
                    relationship_layers[layer].push(relationship.id);
                    break;
                }
            }
        }
        get_person_indices(PersonIndexInput {
            person_layers: layers,
            relationship_layers: &relationship_layers,
            relationships,
            layout_algorithm: LayoutAlgorithm::KinshipExpansion,
        })
    }

    fn column(output: &PersonIndexOutput, pid: Pid) -> usize {
        output
            .person_indices
            .iter()
            .flatten()
            .find(|person| person.pid == pid)
            .expect("person occurrence")
            .index
    }

    #[test]
    fn expands_compactly_below_the_oldest_layer_root() {
        let layers = vec![vec![pid(1), pid(9)], vec![pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![relationship(10, &[1, 2]), relationship(11, &[2, 3])];
        let output = run(&layers, &relationships);
        let root_column = column(&output, pid(1));

        assert_eq!(output.row_length, 3);
        assert_eq!(root_column, 1);
        assert!(column(&output, pid(2)).abs_diff(root_column) <= 1);
        assert!(column(&output, pid(3)).abs_diff(root_column) <= 1);
    }

    #[test]
    fn repeated_people_stay_on_every_layer_and_align_without_collisions() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(2), pid(3)], vec![pid(4)]];
        let relationships = vec![relationship(10, &[1, 2, 3]), relationship(11, &[3, 4])];
        let first = run(&layers, &relationships);
        let second = run(&layers, &relationships);

        let repeated_columns = first
            .person_indices
            .iter()
            .flatten()
            .filter(|person| person.pid == pid(2))
            .map(|person| person.index)
            .collect::<Vec<_>>();
        assert_eq!(repeated_columns.len(), 2);
        assert!(repeated_columns[0].abs_diff(repeated_columns[1]) <= 1);
        assert_eq!(
            first
                .person_indices
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![2, 2, 1]
        );
        for layer in &first.person_indices {
            assert_eq!(
                layer
                    .iter()
                    .map(|person| person.index)
                    .collect::<HashSet<_>>()
                    .len(),
                layer.len()
            );
        }
        assert_eq!(
            first
                .person_indices
                .iter()
                .flatten()
                .map(|person| (person.pid, person.index))
                .collect::<Vec<_>>(),
            second
                .person_indices
                .iter()
                .flatten()
                .map(|person| (person.pid, person.index))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn centers_two_children_below_two_parents() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![Relationship {
            id: RelationshipId(10),
            parents: [Some(pid(1)), Some(pid(2))],
            children: vec![pid(3), pid(4)],
        }];
        let output = run(&layers, &relationships);
        let center_twice = |layer: usize, people: [Pid; 2]| {
            people
                .map(|pid| {
                    output.person_indices[layer]
                        .iter()
                        .find(|person| person.pid == pid)
                        .expect("person occurrence")
                        .index
                })
                .into_iter()
                .sum::<usize>()
        };

        assert_eq!(
            center_twice(0, [pid(1), pid(2)]),
            center_twice(1, [pid(3), pid(4)])
        );
    }

    #[test]
    fn puts_partnered_siblings_outside_unpartnered_siblings() {
        let layers = vec![
            vec![pid(1), pid(2)],
            vec![pid(3), pid(4), pid(5), pid(6), pid(7), pid(8)],
            vec![pid(9), pid(10)],
        ];
        let relationships = vec![
            Relationship {
                id: RelationshipId(10),
                parents: [Some(pid(1)), Some(pid(2))],
                children: vec![pid(3), pid(4), pid(5), pid(6)],
            },
            Relationship {
                id: RelationshipId(11),
                parents: [Some(pid(3)), Some(pid(7))],
                children: vec![pid(9)],
            },
            Relationship {
                id: RelationshipId(12),
                parents: [Some(pid(6)), Some(pid(8))],
                children: vec![pid(10)],
            },
        ];
        let output = run(&layers, &relationships);
        let sibling_column = |pid| {
            output.person_indices[1]
                .iter()
                .find(|person| person.pid == pid)
                .expect("sibling occurrence")
                .index
        };
        let partnered = [sibling_column(pid(3)), sibling_column(pid(6))];
        let outside = [
            *partnered.iter().min().unwrap(),
            *partnered.iter().max().unwrap(),
        ];

        for unpartnered in [pid(4), pid(5)].map(sibling_column) {
            assert!(outside[0] < unpartnered && unpartnered < outside[1]);
        }
    }
}
