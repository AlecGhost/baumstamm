use super::{PersonIndexInput, PersonIndexOutput, common::project_to_slots};
use crate::{
    indices::{self, PersonIndex},
    lines,
};

const MAX_PASSES: usize = 6;
const REPEATED_PERSON_TARGET_WEIGHT: f64 = 4.0;
const PARENT_CHILD_TARGET_WEIGHT: f64 = 12.0;
const SPOUSE_TARGET_WEIGHT: f64 = 16.0;

/// Places fixed-layer person occurrences by minimizing the horizontal
/// connections produced by the same model that renders the grid.
///
/// Candidate layouts are ordered by pairwise horizontal-segment congestion,
/// total horizontal length, and family-center alignment. Stable occurrence
/// coordinates provide deterministic tie-breaking after those objectives.
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

    // More columns cannot improve the secondary length objective and made
    // large trees unnecessarily sparse. Use the smallest collision-free
    // width, but start from the relationship-aware force layout rather than a
    // centered block.
    let row_length = widest_layer;
    let force_seed = compact_force_seed(&input, row_length);
    let centered = centered_indices(input.person_layers, row_length);
    let mut person_indices =
        if score(&input, &force_seed, row_length) < score(&input, &centered, row_length) {
            force_seed
        } else {
            centered
        };
    optimize(&input, &mut person_indices, row_length);

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    congestion: u64,
    length: u64,
    family_alignment: u64,
}

fn optimize(
    input: &PersonIndexInput<'_>,
    person_indices: &mut [Vec<PersonIndex>],
    row_length: usize,
) {
    let mut current_score = score(input, person_indices, row_length);

    for pass in 0..MAX_PASSES {
        let mut changed = false;
        let layer_order = if pass % 2 == 0 {
            (0..person_indices.len()).collect::<Vec<_>>()
        } else {
            (0..person_indices.len()).rev().collect::<Vec<_>>()
        };
        for layer in layer_order {
            if let Some(reordered) = reordered_layer(input, person_indices, layer, row_length) {
                let previous = std::mem::replace(&mut person_indices[layer], reordered);
                let reordered_score = score(input, person_indices, row_length);
                if reordered_score < current_score {
                    current_score = reordered_score;
                    changed = true;
                } else {
                    person_indices[layer] = previous;
                }
            }
            let targets = (0..person_indices[layer].len())
                .map(|person| relationship_target(input, person_indices, layer, person))
                .collect::<Vec<_>>();
            let occupied = person_indices[layer]
                .iter()
                .enumerate()
                .map(|(person, value)| (value.index, person))
                .collect::<std::collections::BTreeMap<_, _>>();
            let mut neighbourhood = Neighbourhood {
                input,
                row_length,
                current_score,
                best_move: None,
            };

            for first in 0..person_indices[layer].len().saturating_sub(1) {
                neighbourhood.consider_swap(person_indices, layer, first, first + 1);
            }
            for (person, target) in targets.into_iter().enumerate() {
                let Some(target) = target else { continue };
                let target_columns = [target.floor(), target.ceil()]
                    .map(|column| column.clamp(0.0, row_length.saturating_sub(1) as f64) as usize);
                for target_column in target_columns {
                    if let Some(other) = occupied.get(&target_column).copied() {
                        if other != person {
                            neighbourhood.consider_swap(person_indices, layer, person, other);
                        }
                    } else {
                        neighbourhood.consider_move(person_indices, layer, person, target_column);
                    }
                }
                if let Some(nearest_gap) = (0..row_length)
                    .filter(|column| !occupied.contains_key(column))
                    .min_by(|first, second| {
                        (*first as f64 - target)
                            .abs()
                            .total_cmp(&(*second as f64 - target).abs())
                            .then_with(|| first.cmp(second))
                    })
                {
                    neighbourhood.consider_move(person_indices, layer, person, nearest_gap);
                }
            }

            if let Some(best_move) = neighbourhood.best_move {
                apply_move(person_indices, best_move.kind);
                current_score = best_move.score;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

#[derive(Clone, Copy)]
enum Move {
    Swap {
        layer: usize,
        first: usize,
        second: usize,
    },
    Move {
        layer: usize,
        person: usize,
        column: usize,
    },
}

struct ScoredMove {
    kind: Move,
    score: Score,
    coordinates: Vec<usize>,
}

struct Neighbourhood<'a, 'input> {
    input: &'a PersonIndexInput<'input>,
    row_length: usize,
    current_score: Score,
    best_move: Option<ScoredMove>,
}

impl Neighbourhood<'_, '_> {
    fn consider_swap(
        &mut self,
        person_indices: &mut [Vec<PersonIndex>],
        layer: usize,
        first: usize,
        second: usize,
    ) {
        let kind = Move::Swap {
            layer,
            first,
            second,
        };
        apply_move(person_indices, kind);
        self.consider_current(person_indices, kind);
        apply_move(person_indices, kind);
    }

    fn consider_move(
        &mut self,
        person_indices: &mut [Vec<PersonIndex>],
        layer: usize,
        person: usize,
        column: usize,
    ) {
        let previous = person_indices[layer][person].index;
        let kind = Move::Move {
            layer,
            person,
            column,
        };
        apply_move(person_indices, kind);
        self.consider_current(person_indices, kind);
        person_indices[layer][person].index = previous;
    }

    fn consider_current(&mut self, person_indices: &[Vec<PersonIndex>], kind: Move) {
        let candidate_score = score(self.input, person_indices, self.row_length);
        if candidate_score >= self.current_score {
            return;
        }
        let candidate = ScoredMove {
            kind,
            score: candidate_score,
            coordinates: coordinate_key(person_indices),
        };
        if self.best_move.as_ref().is_none_or(|best| {
            (candidate.score, &candidate.coordinates) < (best.score, &best.coordinates)
        }) {
            self.best_move = Some(candidate);
        }
    }
}

fn compact_force_seed(input: &PersonIndexInput<'_>, row_length: usize) -> Vec<Vec<PersonIndex>> {
    let force = super::force_directed::get_person_indices(*input);
    force
        .person_indices
        .iter()
        .map(|layer| {
            let mut ordered = layer.iter().enumerate().collect::<Vec<_>>();
            ordered.sort_by_key(|(_, person)| person.index);
            let scale = |index: usize| {
                if force.row_length <= 1 || row_length <= 1 {
                    0.0
                } else {
                    index as f64 * (row_length - 1) as f64 / (force.row_length - 1) as f64
                }
            };
            let targets = ordered
                .iter()
                .map(|(_, person)| scale(person.index))
                .collect::<Vec<_>>();
            let slots = project_to_slots(&targets, row_length);
            let mut result = vec![None; layer.len()];
            for ((original, person), slot) in ordered.into_iter().zip(slots) {
                result[original] = Some(PersonIndex {
                    pid: person.pid,
                    index: slot,
                });
            }
            result
                .into_iter()
                .map(|person| person.expect("every force occurrence is projected"))
                .collect()
        })
        .collect()
}

fn relationship_target(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    layer: usize,
    person: usize,
) -> Option<f64> {
    let pid = person_indices[layer][person].pid;
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut add = |column: f64, weight: f64| {
        weighted_sum += column * weight;
        total_weight += weight;
    };

    for (other_layer, occurrences) in person_indices.iter().enumerate() {
        if other_layer != layer {
            for occurrence in occurrences
                .iter()
                .filter(|occurrence| occurrence.pid == pid)
            {
                add(occurrence.index as f64, REPEATED_PERSON_TARGET_WEIGHT);
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
                let parents = relationship
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
                if let (Some(minimum), Some(maximum)) = (parents.iter().min(), parents.iter().max())
                {
                    add((minimum + maximum) as f64 / 2.0, PARENT_CHILD_TARGET_WEIGHT);
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
                    let child_columns = relationship
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
                        (child_columns.iter().min(), child_columns.iter().max())
                    {
                        add((minimum + maximum) as f64 / 2.0, PARENT_CHILD_TARGET_WEIGHT);
                    }
                }
                for partner in relationship
                    .parents
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|partner| *partner != pid)
                {
                    if let Some(partner) = person_indices[layer]
                        .iter()
                        .find(|person| person.pid == partner)
                    {
                        add(partner.index as f64, SPOUSE_TARGET_WEIGHT);
                    }
                }
            }
        }
    }

    (total_weight > 0.0).then_some(weighted_sum / total_weight)
}

fn reordered_layer(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    layer: usize,
    row_length: usize,
) -> Option<Vec<PersonIndex>> {
    if person_indices[layer].len() < 2 {
        return None;
    }
    let mut people = person_indices[layer]
        .iter()
        .enumerate()
        .map(|(order, person)| {
            (
                person.clone(),
                relationship_target(input, person_indices, layer, order)
                    .unwrap_or(person.index as f64),
                order,
            )
        })
        .collect::<Vec<_>>();
    people.sort_by(|first, second| {
        first
            .1
            .total_cmp(&second.1)
            .then_with(|| first.2.cmp(&second.2))
            .then_with(|| first.0.pid.cmp(&second.0.pid))
    });
    let targets = people.iter().map(|person| person.1).collect::<Vec<_>>();
    let slots = project_to_slots(&targets, row_length);
    Some(
        people
            .into_iter()
            .zip(slots)
            .map(|((mut person, _, _), index)| {
                person.index = index;
                person
            })
            .collect(),
    )
}

fn apply_move(person_indices: &mut [Vec<PersonIndex>], kind: Move) {
    match kind {
        Move::Swap {
            layer,
            first,
            second,
        } => {
            let first_column = person_indices[layer][first].index;
            person_indices[layer][first].index = person_indices[layer][second].index;
            person_indices[layer][second].index = first_column;
        }
        Move::Move {
            layer,
            person,
            column,
        } => person_indices[layer][person].index = column,
    }
}

fn centered_indices(
    person_layers: &[Vec<baumstamm_lib::PersonId>],
    row_length: usize,
) -> Vec<Vec<PersonIndex>> {
    person_layers
        .iter()
        .map(|layer| {
            let start = (row_length - layer.len()) / 2;
            layer
                .iter()
                .enumerate()
                .map(|(offset, pid)| PersonIndex {
                    index: start + offset,
                    pid: *pid,
                })
                .collect()
        })
        .collect()
}

fn coordinate_key(person_indices: &[Vec<PersonIndex>]) -> Vec<usize> {
    person_indices
        .iter()
        .flatten()
        .map(|person| person.index)
        .collect()
}

fn score(
    input: &PersonIndexInput<'_>,
    person_indices: &[Vec<PersonIndex>],
    row_length: usize,
) -> Score {
    let rel_indices = indices::get_rel_indices(
        input.relationship_layers,
        input.relationships,
        person_indices,
    );
    let mut congestion = 0_u64;
    let mut length = 0_u64;
    let mut family_alignment = 0_u64;

    for row in &rel_indices {
        for relation in row {
            let parents = relation.get_parents();
            if !parents.is_empty() && !relation.children.is_empty() {
                family_alignment = family_alignment.saturating_add(
                    center_twice(&parents).abs_diff(center_twice(&relation.children)) as u64,
                );
            }
        }
        for channel in lines::create_horizontal(row) {
            let mut occupation = vec![0_u64; row_length];
            for line in channel {
                length = length.saturating_add(line.end.abs_diff(line.start) as u64);
                for count in &mut occupation[line.start..=line.end] {
                    congestion = congestion.saturating_add(*count);
                    *count += 1;
                }
            }
        }
    }

    Score {
        congestion,
        length,
        family_alignment,
    }
}

fn center_twice(columns: &[usize]) -> usize {
    let minimum = columns.iter().min().copied().unwrap_or_default();
    let maximum = columns.iter().max().copied().unwrap_or_default();
    minimum + maximum
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use baumstamm_lib::{PersonId, Relationship, RelationshipId};
    use std::collections::BTreeSet;

    fn pid(value: u128) -> PersonId {
        value.into()
    }

    fn relationship(id: u128, parents: [u128; 2], children: [u128; 2]) -> Relationship {
        Relationship {
            id: RelationshipId(id),
            parents: parents.map(pid).map(Some),
            children: children.map(pid).to_vec(),
        }
    }

    fn input<'a>(
        person_layers: &'a Vec<Vec<PersonId>>,
        relationship_layers: &'a Vec<Vec<RelationshipId>>,
        relationships: &'a [Relationship],
    ) -> PersonIndexInput<'a> {
        PersonIndexInput {
            person_layers,
            relationship_layers,
            relationships,
            layout_algorithm: LayoutAlgorithm::ConnectionOptimized,
        }
    }

    #[test]
    fn separates_interleaved_connection_segments() {
        let people = vec![
            vec![pid(1), pid(3), pid(2), pid(4)],
            vec![pid(5), pid(7), pid(6), pid(8)],
        ];
        let relationships = vec![
            relationship(10, [1, 2], [5, 6]),
            relationship(11, [3, 4], [7, 8]),
        ];
        let relationship_layers = vec![vec![], vec![RelationshipId(10), RelationshipId(11)]];
        let request = input(&people, &relationship_layers, &relationships);
        let deliberate_worse = centered_indices(&people, 4);
        let worse_score = score(&request, &deliberate_worse, 4);

        let output = get_person_indices(request);
        let optimized_score = score(&request, &output.person_indices, output.row_length);

        assert!(optimized_score < worse_score);
        assert_eq!(optimized_score.congestion, 0);
    }

    #[test]
    fn is_deterministic_collision_free_and_keeps_repeated_occurrences() {
        let people = vec![
            vec![pid(1), pid(3), pid(2), pid(4)],
            vec![pid(5), pid(3), pid(6)],
        ];
        let relationships = vec![relationship(10, [1, 2], [5, 6])];
        let relationship_layers = vec![vec![], vec![RelationshipId(10)]];
        let first = get_person_indices(input(&people, &relationship_layers, &relationships));
        let second = get_person_indices(input(&people, &relationship_layers, &relationships));

        assert_eq!(first.row_length, second.row_length);
        assert_eq!(
            coordinate_key(&first.person_indices),
            coordinate_key(&second.person_indices)
        );
        assert_eq!(
            first
                .person_indices
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![4, 3]
        );
        assert_eq!(
            first
                .person_indices
                .iter()
                .flatten()
                .filter(|person| person.pid == pid(3))
                .count(),
            2
        );
        for layer in &first.person_indices {
            let columns = layer
                .iter()
                .map(|person| person.index)
                .collect::<BTreeSet<_>>();
            assert_eq!(columns.len(), layer.len());
            assert!(columns.iter().all(|column| *column < first.row_length));
        }
    }

    #[test]
    fn equal_length_family_layouts_prefer_matching_centers() {
        let people = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![relationship(10, [1, 2], [3, 4])];
        let relationship_layers = vec![vec![], vec![RelationshipId(10)]];
        let request = input(&people, &relationship_layers, &relationships);
        let aligned = vec![
            vec![
                PersonIndex {
                    pid: pid(1),
                    index: 1,
                },
                PersonIndex {
                    pid: pid(2),
                    index: 2,
                },
            ],
            vec![
                PersonIndex {
                    pid: pid(3),
                    index: 1,
                },
                PersonIndex {
                    pid: pid(4),
                    index: 2,
                },
            ],
        ];
        let shifted = vec![
            aligned[0].clone(),
            vec![
                PersonIndex {
                    pid: pid(3),
                    index: 0,
                },
                PersonIndex {
                    pid: pid(4),
                    index: 1,
                },
            ],
        ];

        let aligned_score = score(&request, &aligned, 4);
        let shifted_score = score(&request, &shifted, 4);
        assert_eq!(aligned_score.congestion, shifted_score.congestion);
        assert_eq!(aligned_score.length, shifted_score.length);
        assert!(aligned_score.family_alignment < shifted_score.family_alignment);
        assert!(aligned_score < shifted_score);
    }
}
