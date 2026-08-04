use super::PersonIndexInput;
use crate::{Grid, indices::PersonIndex};
use baumstamm_lib::{PersonId, Relationship, RelationshipId};
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

type Pid = PersonId;
type Rid = RelationshipId;
type PositionLayer = HashMap<Pid, usize>;

const MAX_INSERTION_SWEEPS: usize = 8;

/// A strict priority list. A lower-priority field can never compensate for a
/// regression in a higher-priority field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct LayoutScore {
    crossings: u64,
    connector_length: u64,
    interleavings: u64,
    displacement: u64,
    off_center: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct BandScore {
    crossings: u64,
    connector_length: u64,
}

struct LayerBaseline {
    bands: Vec<(usize, BandScore)>,
    interleavings: u64,
    displacement: u64,
    off_center: u64,
}

#[derive(Clone)]
struct BandRelationship {
    parents: [Option<Pid>; 2],
    children: Vec<Pid>,
}

#[derive(Clone)]
struct FamilyGroup {
    members: HashSet<Pid>,
}

#[derive(Clone, Copy)]
struct Center {
    numerator: u64,
    denominator: u64,
}

#[derive(Clone)]
struct ConnectorGeometry {
    parent_center: Option<Center>,
    child_center: Option<Center>,
    connector_length: u64,
}

struct Evaluator {
    bands: Vec<Vec<BandRelationship>>,
    groups: Vec<Vec<FamilyGroup>>,
    anchors: Vec<HashMap<Pid, usize>>,
    row_length: usize,
}

impl Evaluator {
    fn new(
        person_layers: &Grid<Pid>,
        relationship_layers: &Grid<Rid>,
        relationships: &[Relationship],
        row_length: usize,
    ) -> Self {
        let relationships_by_id = relationships
            .iter()
            .map(|relationship| (relationship.id, relationship))
            .collect::<HashMap<_, _>>();
        let bands = relationship_layers
            .iter()
            .map(|relationship_ids| {
                relationship_ids
                    .iter()
                    .map(|relationship_id| {
                        let relationship = relationships_by_id
                            .get(relationship_id)
                            .expect("Relationship layer must reference an existing relationship");
                        BandRelationship {
                            parents: relationship.parents,
                            children: relationship.children.clone(),
                        }
                    })
                    .collect()
            })
            .collect();

        let mut groups = vec![Vec::new(); person_layers.len()];
        for (child_layer, relationship_ids) in relationship_layers.iter().enumerate() {
            for relationship_id in relationship_ids {
                let relationship = relationships_by_id
                    .get(relationship_id)
                    .expect("Relationship layer must reference an existing relationship");
                if relationship.children.len() >= 2 && child_layer < groups.len() {
                    groups[child_layer].push(FamilyGroup {
                        members: relationship.children.iter().copied().collect(),
                    });
                }
                if child_layer > 0 {
                    let parents = relationship
                        .parents
                        .iter()
                        .flatten()
                        .copied()
                        .collect::<Vec<_>>();
                    if parents.len() >= 2 && child_layer - 1 < groups.len() {
                        groups[child_layer - 1].push(FamilyGroup {
                            members: parents.into_iter().collect(),
                        });
                    }
                }
            }
        }

        let anchors = person_layers
            .iter()
            .map(|people| {
                people
                    .iter()
                    .enumerate()
                    .map(|(index, person)| {
                        (*person, spread_position(index, people.len(), row_length))
                    })
                    .collect()
            })
            .collect();

        Self {
            bands,
            groups,
            anchors,
            row_length,
        }
    }

    fn score(&self, positions: &[PositionLayer]) -> LayoutScore {
        let mut score = LayoutScore::default();
        for band_index in 0..self.bands.len() {
            let band = self.band_score(positions, band_index, None);
            score.crossings += band.crossings;
            score.connector_length += band.connector_length;
        }
        for layer_index in 0..positions.len() {
            score.interleavings += self.layer_interleavings(positions, layer_index, None);
            score.displacement += self.layer_displacement(positions, layer_index, None);
            score.off_center += self.layer_off_center(positions, layer_index, None);
        }
        score
    }

    fn score_with_layer(
        &self,
        positions: &[PositionLayer],
        layer_index: usize,
        candidate: &PositionLayer,
        current: LayoutScore,
        baseline: &LayerBaseline,
    ) -> LayoutScore {
        let mut score = current;
        for (band_index, old) in &baseline.bands {
            let new = self.band_score(positions, *band_index, Some((layer_index, candidate)));
            score.crossings = score.crossings - old.crossings + new.crossings;
            score.connector_length =
                score.connector_length - old.connector_length + new.connector_length;
        }

        let new_interleavings = self.layer_interleavings(positions, layer_index, Some(candidate));
        score.interleavings = score.interleavings - baseline.interleavings + new_interleavings;

        let new_displacement = self.layer_displacement(positions, layer_index, Some(candidate));
        score.displacement = score.displacement - baseline.displacement + new_displacement;

        let new_off_center = self.layer_off_center(positions, layer_index, Some(candidate));
        score.off_center = score.off_center - baseline.off_center + new_off_center;
        score
    }

    fn layer_baseline(&self, positions: &[PositionLayer], layer_index: usize) -> LayerBaseline {
        let bands = [layer_index, layer_index + 1]
            .into_iter()
            .filter(|band_index| *band_index < self.bands.len())
            .map(|band_index| (band_index, self.band_score(positions, band_index, None)))
            .collect();
        LayerBaseline {
            bands,
            interleavings: self.layer_interleavings(positions, layer_index, None),
            displacement: self.layer_displacement(positions, layer_index, None),
            off_center: self.layer_off_center(positions, layer_index, None),
        }
    }

    fn insertion_targets(
        &self,
        positions: &[PositionLayer],
        layer_index: usize,
        person: Pid,
    ) -> Vec<usize> {
        let mut focal_positions = vec![
            self.anchors[layer_index][&person],
            self.row_length.saturating_sub(1) / 2,
        ];

        if layer_index < self.bands.len() {
            for relationship in &self.bands[layer_index] {
                if !relationship.children.contains(&person) {
                    continue;
                }
                focal_positions.extend(
                    relationship
                        .children
                        .iter()
                        .filter_map(|child| position_at(positions, layer_index, *child, None)),
                );
                if let Some(parent_layer) = layer_index.checked_sub(1) {
                    focal_positions.extend(
                        relationship.parents.iter().flatten().filter_map(|parent| {
                            position_at(positions, parent_layer, *parent, None)
                        }),
                    );
                }
            }
        }

        if let Some(relationship_band) = self.bands.get(layer_index + 1) {
            for relationship in relationship_band {
                if !relationship
                    .parents
                    .iter()
                    .flatten()
                    .any(|parent| *parent == person)
                {
                    continue;
                }
                focal_positions.extend(
                    relationship
                        .parents
                        .iter()
                        .flatten()
                        .filter_map(|parent| position_at(positions, layer_index, *parent, None)),
                );
                if layer_index + 1 < positions.len() {
                    focal_positions.extend(
                        relationship.children.iter().filter_map(|child| {
                            position_at(positions, layer_index + 1, *child, None)
                        }),
                    );
                }
            }
        }

        let mut targets = focal_positions
            .into_iter()
            .flat_map(|position| {
                [
                    position.saturating_sub(1),
                    position,
                    position.saturating_add(1),
                ]
            })
            .filter(|position| *position < self.row_length)
            .collect::<Vec<_>>();
        targets.sort_unstable();
        targets.dedup();
        targets
    }

    fn band_score(
        &self,
        positions: &[PositionLayer],
        band_index: usize,
        replacement: Option<(usize, &PositionLayer)>,
    ) -> BandScore {
        let geometries = self.bands[band_index]
            .iter()
            .map(|relationship| {
                self.connector_geometry(positions, band_index, relationship, replacement)
            })
            .collect::<Vec<_>>();
        let connector_length = geometries
            .iter()
            .map(|geometry| geometry.connector_length)
            .sum();
        let mut crossings = 0;
        for first in 0..geometries.len() {
            for second in (first + 1)..geometries.len() {
                let first_geometry = &geometries[first];
                let second_geometry = &geometries[second];
                let (
                    Some(first_parent),
                    Some(first_child),
                    Some(second_parent),
                    Some(second_child),
                ) = (
                    first_geometry.parent_center,
                    first_geometry.child_center,
                    second_geometry.parent_center,
                    second_geometry.child_center,
                )
                else {
                    continue;
                };
                let parent_order = compare_centers(first_parent, second_parent);
                let child_order = compare_centers(first_child, second_child);
                if matches!(
                    (parent_order, child_order),
                    (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
                ) {
                    crossings += 1;
                }
            }
        }
        BandScore {
            crossings,
            connector_length,
        }
    }

    fn connector_geometry(
        &self,
        positions: &[PositionLayer],
        child_layer: usize,
        relationship: &BandRelationship,
        replacement: Option<(usize, &PositionLayer)>,
    ) -> ConnectorGeometry {
        let parent_positions = child_layer
            .checked_sub(1)
            .into_iter()
            .flat_map(|parent_layer| {
                relationship
                    .parents
                    .iter()
                    .flatten()
                    .filter_map(move |parent| {
                        position_at(positions, parent_layer, *parent, replacement)
                            .map(|position| position as u64 * 2)
                    })
            })
            .collect::<Vec<_>>();
        let child_positions = relationship
            .children
            .iter()
            .filter_map(|child| {
                position_at(positions, child_layer, *child, replacement)
                    .map(|position| position as u64 * 2)
            })
            .collect::<Vec<_>>();

        let parent_center = center(&parent_positions);
        let child_center = center(&child_positions);
        let parent_span = span(&parent_positions);
        let trunk = parent_center.map(|center| center.numerator / center.denominator);
        let mut child_bus_points = child_positions.clone();
        if let Some(trunk) = trunk {
            child_bus_points.push(trunk);
        }
        let child_span = span(&child_bus_points);

        ConnectorGeometry {
            parent_center,
            child_center,
            connector_length: parent_span + child_span,
        }
    }

    fn layer_interleavings(
        &self,
        positions: &[PositionLayer],
        layer_index: usize,
        replacement: Option<&PositionLayer>,
    ) -> u64 {
        let layer = replacement.unwrap_or(&positions[layer_index]);
        self.groups[layer_index]
            .iter()
            .map(|group| {
                let member_positions = group
                    .members
                    .iter()
                    .filter_map(|member| layer.get(member).copied())
                    .collect::<Vec<_>>();
                if member_positions.len() != group.members.len() || member_positions.len() < 2 {
                    return 0;
                }
                let first = *member_positions.iter().min().expect("family position");
                let last = *member_positions.iter().max().expect("family position");
                layer
                    .iter()
                    .filter(|(person, position)| {
                        first < **position && **position < last && !group.members.contains(person)
                    })
                    .count() as u64
            })
            .sum()
    }

    fn layer_displacement(
        &self,
        positions: &[PositionLayer],
        layer_index: usize,
        replacement: Option<&PositionLayer>,
    ) -> u64 {
        let layer = replacement.unwrap_or(&positions[layer_index]);
        layer
            .iter()
            .map(|(person, position)| position.abs_diff(self.anchors[layer_index][person]) as u64)
            .sum()
    }

    fn layer_off_center(
        &self,
        positions: &[PositionLayer],
        layer_index: usize,
        replacement: Option<&PositionLayer>,
    ) -> u64 {
        let layer = replacement.unwrap_or(&positions[layer_index]);
        let position_sum = layer.values().sum::<usize>();
        (position_sum.saturating_mul(2) as u64).abs_diff(
            layer
                .len()
                .saturating_mul(self.row_length.saturating_sub(1)) as u64,
        )
    }
}

/// Standalone placement engine for the lexicographic objective. It deliberately
/// shares neither initialization, attraction weights, nor refinement machinery
/// with the centered and relationship-force layouts.
pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> Grid<PersonIndex> {
    let evaluator = Evaluator::new(
        input.person_layers,
        input.relationship_layers,
        input.relationships,
        input.row_length,
    );
    let mut indices = initial_positions(input.person_layers, input.row_length);
    let mut positions = position_layers(&indices);
    let mut score = evaluator.score(&positions);

    for sweep in 0..MAX_INSERTION_SWEEPS {
        let mut changed = false;
        if sweep % 2 == 0 {
            for layer_index in 0..indices.len() {
                if let Some((candidate_indices, candidate_positions, candidate_score)) =
                    best_insertion(&evaluator, &indices, &positions, layer_index, score)
                {
                    indices[layer_index] = candidate_indices;
                    positions[layer_index] = candidate_positions;
                    score = candidate_score;
                    changed = true;
                }
            }
        } else {
            for layer_index in (0..indices.len()).rev() {
                if let Some((candidate_indices, candidate_positions, candidate_score)) =
                    best_insertion(&evaluator, &indices, &positions, layer_index, score)
                {
                    indices[layer_index] = candidate_indices;
                    positions[layer_index] = candidate_positions;
                    score = candidate_score;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    indices
}

fn initial_positions(person_layers: &Grid<Pid>, row_length: usize) -> Grid<PersonIndex> {
    person_layers
        .iter()
        .map(|people| {
            people
                .iter()
                .enumerate()
                .map(|(index, person)| PersonIndex {
                    index: spread_position(index, people.len(), row_length),
                    pid: *person,
                })
                .collect()
        })
        .collect()
}

fn spread_position(index: usize, people: usize, row_length: usize) -> usize {
    match people {
        0 => 0,
        1 => row_length.saturating_sub(1) / 2,
        _ => {
            let intervals = people - 1;
            (index * row_length.saturating_sub(1) + intervals / 2) / intervals
        }
    }
}

fn best_insertion(
    evaluator: &Evaluator,
    indices: &Grid<PersonIndex>,
    positions: &[PositionLayer],
    layer_index: usize,
    current_score: LayoutScore,
) -> Option<(Vec<PersonIndex>, PositionLayer, LayoutScore)> {
    let mut people_by_position = indices[layer_index].clone();
    people_by_position.sort_by_key(|person| person.index);
    let mut best = None;
    let mut best_score = current_score;
    let baseline = evaluator.layer_baseline(positions, layer_index);

    for person in people_by_position {
        for target in evaluator.insertion_targets(positions, layer_index, person.pid) {
            if target == person.index {
                continue;
            }
            let candidate_indices = insert_at_slot(
                &indices[layer_index],
                person.pid,
                target,
                evaluator.row_length,
            );
            let candidate_positions = candidate_indices
                .iter()
                .map(|person| (person.pid, person.index))
                .collect::<PositionLayer>();
            let candidate_score = evaluator.score_with_layer(
                positions,
                layer_index,
                &candidate_positions,
                current_score,
                &baseline,
            );
            if candidate_score < best_score {
                best_score = candidate_score;
                best = Some((candidate_indices, candidate_positions, candidate_score));
            }
        }
    }
    best
}

fn insert_at_slot(
    layer: &[PersonIndex],
    person: Pid,
    target: usize,
    row_length: usize,
) -> Vec<PersonIndex> {
    let mut slots = vec![None; row_length];
    let mut people = Vec::with_capacity(layer.len());
    for item in layer {
        let person_index = people.len();
        people.push(item.pid);
        slots[item.index] = Some(person_index);
    }
    let moving_index = people
        .iter()
        .position(|candidate| *candidate == person)
        .expect("Inserted person must exist");
    let source = slots
        .iter()
        .position(|occupant| *occupant == Some(moving_index))
        .expect("Inserted person must occupy a slot");

    if slots[target].is_none() {
        slots[source] = None;
        slots[target] = Some(moving_index);
    } else if target < source {
        for slot in (target..source).rev() {
            slots[slot + 1] = slots[slot];
        }
        slots[target] = Some(moving_index);
    } else {
        for slot in source..target {
            slots[slot] = slots[slot + 1];
        }
        slots[target] = Some(moving_index);
    }

    let positions = slots
        .iter()
        .enumerate()
        .filter_map(|(position, occupant)| occupant.map(|occupant| (occupant, position)))
        .collect::<HashMap<_, _>>();
    people
        .into_iter()
        .enumerate()
        .map(|(index, pid)| PersonIndex {
            index: positions[&index],
            pid,
        })
        .collect()
}

fn position_layers(indices: &Grid<PersonIndex>) -> Vec<PositionLayer> {
    indices
        .iter()
        .map(|layer| {
            layer
                .iter()
                .map(|person| (person.pid, person.index))
                .collect()
        })
        .collect()
}

fn position_at(
    positions: &[PositionLayer],
    layer_index: usize,
    person: Pid,
    replacement: Option<(usize, &PositionLayer)>,
) -> Option<usize> {
    replacement
        .filter(|(replacement_layer, _)| *replacement_layer == layer_index)
        .map(|(_, layer)| layer)
        .unwrap_or(&positions[layer_index])
        .get(&person)
        .copied()
}

fn center(positions: &[u64]) -> Option<Center> {
    (!positions.is_empty()).then(|| Center {
        numerator: positions.iter().sum(),
        denominator: positions.len() as u64,
    })
}

fn compare_centers(first: Center, second: Center) -> Ordering {
    (first.numerator as u128 * second.denominator as u128)
        .cmp(&(second.numerator as u128 * first.denominator as u128))
}

fn span(positions: &[u64]) -> u64 {
    let Some(first) = positions.iter().min() else {
        return 0;
    };
    let last = positions.iter().max().expect("non-empty positions");
    last - first
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(value: u128) -> Pid {
        value.into()
    }

    fn rid(value: u128) -> Rid {
        value.into()
    }

    fn relationship(id: u128, parents: [Option<u128>; 2], children: &[u128]) -> Relationship {
        Relationship {
            id: rid(id),
            parents: parents.map(|parent| parent.map(pid)),
            children: children.iter().copied().map(pid).collect(),
        }
    }

    #[test]
    fn initial_layout_uses_the_available_breathing_room() {
        let positions = initial_positions(&vec![vec![pid(1), pid(2), pid(3), pid(4)]], 5);
        let slots = positions[0]
            .iter()
            .map(|person| person.index)
            .collect::<Vec<_>>();

        assert_eq!(slots, vec![0, 1, 3, 4]);
    }

    #[test]
    fn insertion_preserves_people_and_unique_bounded_slots() {
        let layer = vec![
            PersonIndex {
                pid: pid(1),
                index: 0,
            },
            PersonIndex {
                pid: pid(2),
                index: 2,
            },
            PersonIndex {
                pid: pid(3),
                index: 4,
            },
        ];

        let moved = insert_at_slot(&layer, pid(1), 4, 5);
        let mut slots = moved.iter().map(|person| person.index).collect::<Vec<_>>();
        slots.sort_unstable();

        assert_eq!(slots, vec![1, 3, 4]);
        assert_eq!(
            moved
                .iter()
                .map(|person| person.pid)
                .collect::<HashSet<_>>(),
            layer.iter().map(|person| person.pid).collect()
        );

        for person in &layer {
            for target in 0..5 {
                let candidate = insert_at_slot(&layer, person.pid, target, 5);
                assert!(candidate.iter().all(|person| person.index < 5));
                assert_eq!(
                    candidate
                        .iter()
                        .map(|person| person.pid)
                        .collect::<HashSet<_>>()
                        .len(),
                    layer.len()
                );
                assert_eq!(
                    candidate
                        .iter()
                        .map(|person| person.index)
                        .collect::<HashSet<_>>()
                        .len(),
                    layer.len()
                );
            }
        }
    }

    #[test]
    fn every_score_field_has_strict_lexicographic_priority() {
        let base = LayoutScore {
            crossings: 0,
            connector_length: 0,
            interleavings: 0,
            displacement: 0,
            off_center: 0,
        };
        let worse_crossings = LayoutScore {
            crossings: 1,
            ..base
        };
        let worse_length = LayoutScore {
            connector_length: 1,
            ..base
        };
        let worse_interleavings = LayoutScore {
            interleavings: 1,
            ..base
        };
        let worse_displacement = LayoutScore {
            displacement: 1,
            ..base
        };
        let worse_centering = LayoutScore {
            off_center: 1,
            ..base
        };

        assert!(worse_centering < worse_displacement);
        assert!(worse_displacement < worse_interleavings);
        assert!(worse_interleavings < worse_length);
        assert!(worse_length < worse_crossings);
    }

    #[test]
    fn crossing_count_has_strict_priority_over_connector_length() {
        let person_layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![
            relationship(1, [Some(1), None], &[4]),
            relationship(2, [Some(2), None], &[3]),
        ];
        let relationship_layers = vec![vec![], vec![rid(1), rid(2)]];
        let evaluator = Evaluator::new(&person_layers, &relationship_layers, &relationships, 5);
        let crossed = vec![
            HashMap::from([(pid(1), 1), (pid(2), 2)]),
            HashMap::from([(pid(3), 1), (pid(4), 2)]),
        ];
        let uncrossed_but_longer = vec![
            HashMap::from([(pid(1), 0), (pid(2), 1)]),
            HashMap::from([(pid(3), 4), (pid(4), 3)]),
        ];

        let crossed_score = evaluator.score(&crossed);
        let uncrossed_score = evaluator.score(&uncrossed_but_longer);

        assert_eq!(crossed_score.crossings, 1);
        assert_eq!(uncrossed_score.crossings, 0);
        assert!(
            uncrossed_score.connector_length > crossed_score.connector_length,
            "the lower-priority connector metric must genuinely disagree"
        );
        assert!(uncrossed_score < crossed_score);
    }

    #[test]
    fn optimizer_removes_a_simple_relationship_crossing() {
        let person_layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![
            relationship(1, [Some(1), None], &[4]),
            relationship(2, [Some(2), None], &[3]),
        ];
        let relationship_layers = vec![vec![], vec![rid(1), rid(2)]];
        let indices = get_person_indices(PersonIndexInput {
            person_layers: &person_layers,
            relationship_layers: &relationship_layers,
            relationships: &relationships,
            row_length: 3,
            layout_algorithm: super::super::LayoutAlgorithm::Lexicographic,
        });
        let evaluator = Evaluator::new(&person_layers, &relationship_layers, &relationships, 3);

        assert_eq!(evaluator.score(&position_layers(&indices)).crossings, 0);
        for layer in indices {
            assert!(layer.iter().all(|person| person.index < 3));
            assert_eq!(
                layer
                    .iter()
                    .map(|person| person.index)
                    .collect::<HashSet<_>>()
                    .len(),
                layer.len()
            );
        }
    }
}
