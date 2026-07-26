use super::Grid;
use crate::LayoutAlgorithm;
use baumstamm_lib::Relationship;
use itertools::Itertools;
use std::collections::HashMap;

type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

#[derive(Clone, Debug)]
pub struct RelIndices {
    pub rid: Rid,
    pub parents: [Option<usize>; 2],
    pub children: Vec<usize>,
    pub crossing_point: Option<usize>,
}

impl RelIndices {
    pub fn get_parents(&self) -> Vec<usize> {
        self.parents.iter().flatten().cloned().collect_vec()
    }
}

#[derive(Clone, Debug)]
pub struct PersonIndex {
    pub index: usize,
    pub pid: Pid,
}

pub fn get_rel_indices(
    layers: &Grid<Rid>,
    rels: &[Relationship],
    person_indices: &Grid<PersonIndex>,
) -> Grid<RelIndices> {
    layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            layer
                .iter()
                .map(|rid| {
                    let rel = rels
                        .iter()
                        .find(|rel| rel.id == *rid)
                        .expect("Inconsistent relationships");
                    let mut rel_indices = RelIndices {
                        rid: rel.id,
                        parents: [None, None],
                        children: Vec::new(),
                        crossing_point: None,
                    };
                    if index > 0 {
                        if let Some(parent_indices) = person_indices.get(index - 1) {
                            let mut parent_indices = rel.parents.map(|opt_parent| {
                                opt_parent.and_then(|parent| {
                                    parent_indices
                                        .iter()
                                        .find(|pi| pi.pid == parent)
                                        .map(|pi| pi.index)
                                })
                            });
                            parent_indices.sort();
                            rel_indices.parents = parent_indices;

                            // crossing point is only necessary, if there are children
                            if !rel.children.is_empty() {
                                rel_indices.crossing_point = match parent_indices {
                                    [Some(a), Some(b)] => Some(a + middle(a, b)),
                                    [Some(a), None] => Some(a),
                                    [None, Some(b)] => Some(b),
                                    [None, None] => None,
                                }
                            }
                        }
                    }
                    if let Some(child_indices) = person_indices.get(index) {
                        let children_indices = rel
                            .children
                            .iter()
                            .filter_map(|child| child_indices.iter().find(|pi| pi.pid == *child))
                            .map(|pi| pi.index)
                            .sorted()
                            .collect_vec();
                        if children_indices.len() == rel.children.len() {
                            rel_indices.children = children_indices;
                        }
                    }
                    rel_indices
                })
                // sort by first parent reversed, because None < Some(x)
                // this way colors are sorted from left to right
                .sorted_by(|a, b| b.parents[0].cmp(&a.parents[0]))
                .collect_vec()
        })
        .collect_vec()
}

pub fn get_person_indices(
    person_layers: &Grid<Pid>,
    relationships: &[Relationship],
    row_length: usize,
    layout_algorithm: LayoutAlgorithm,
) -> Grid<PersonIndex> {
    match layout_algorithm {
        LayoutAlgorithm::Centered => get_centered_person_indices(person_layers, row_length),
        LayoutAlgorithm::ForceDirected => {
            get_force_directed_person_indices(person_layers, relationships, row_length)
        }
    }
}

fn get_centered_person_indices(person_layers: &Grid<Pid>, row_length: usize) -> Grid<PersonIndex> {
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

#[derive(Clone, Copy)]
struct Attraction {
    people: [Pid; 2],
    weight: usize,
}

const MARRIAGE_WEIGHT: usize = 12;
const PARENT_CHILD_WEIGHT: usize = 5;
const SIBLING_WEIGHT: usize = 3;
const REFINEMENT_PASSES: usize = 12;

fn get_force_directed_person_indices(
    person_layers: &Grid<Pid>,
    relationships: &[Relationship],
    row_length: usize,
) -> Grid<PersonIndex> {
    let mut positions = get_centered_person_indices(person_layers, row_length);
    let attractions = attractions(relationships);
    let incident_attractions = incident_attractions(&attractions);

    for pass in 0..REFINEMENT_PASSES {
        let layer_indices: Vec<usize> = if pass % 2 == 0 {
            (0..positions.len()).collect()
        } else {
            (0..positions.len()).rev().collect()
        };
        let mut changed = false;
        for layer_index in layer_indices {
            changed |= refine_layer(
                &mut positions,
                layer_index,
                row_length,
                &attractions,
                &incident_attractions,
            );
        }
        if !changed {
            break;
        }
    }

    resolve_child_spouse_order(&mut positions, relationships);
    positions
}

fn attractions(relationships: &[Relationship]) -> Vec<Attraction> {
    let mut attractions = Vec::new();
    for relationship in relationships {
        if let [Some(first), Some(second)] = relationship.parents {
            attractions.push(Attraction {
                people: [first, second],
                weight: MARRIAGE_WEIGHT,
            });
        }
        for [first, second] in relationship
            .children
            .iter()
            .copied()
            .tuple_combinations()
            .map(|(first, second)| [first, second])
        {
            attractions.push(Attraction {
                people: [first, second],
                weight: SIBLING_WEIGHT,
            });
        }
        for parent in relationship.parents.iter().flatten() {
            for child in &relationship.children {
                attractions.push(Attraction {
                    people: [*parent, *child],
                    weight: PARENT_CHILD_WEIGHT,
                });
            }
        }
    }
    attractions
}

fn incident_attractions(attractions: &[Attraction]) -> HashMap<Pid, Vec<usize>> {
    let mut incident = HashMap::<Pid, Vec<usize>>::new();
    for (index, attraction) in attractions.iter().enumerate() {
        for person in attraction.people {
            incident.entry(person).or_default().push(index);
        }
    }
    incident
}

fn position_map(person_indices: &Grid<PersonIndex>) -> HashMap<Pid, usize> {
    person_indices
        .iter()
        .flatten()
        .map(|person| (person.pid, person.index))
        .collect()
}

fn refine_layer(
    person_indices: &mut Grid<PersonIndex>,
    layer_index: usize,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<Pid, Vec<usize>>,
) -> bool {
    let mut changed = false;
    let mut positions = position_map(person_indices);

    for person_index in 0..person_indices[layer_index].len() {
        let person = person_indices[layer_index][person_index].pid;
        let current = person_indices[layer_index][person_index].index;
        let mut best_improvement = 0;
        let mut best_move = None;

        for target in 0..row_length {
            if target == current {
                continue;
            }
            let other_index = person_indices[layer_index]
                .iter()
                .position(|candidate| candidate.index == target);
            let other = other_index.map(|index| person_indices[layer_index][index].pid);
            let mut affected = incident_attractions
                .get(&person)
                .into_iter()
                .flatten()
                .copied()
                .collect_vec();
            if let Some(other) = other {
                affected.extend(
                    incident_attractions
                        .get(&other)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            }
            affected.sort_unstable();
            affected.dedup();

            let before = affected
                .iter()
                .filter_map(|index| attraction_score(&attractions[*index], &positions, &[]))
                .sum::<usize>();
            let mut moved = vec![(person, target)];
            if let Some(other) = other {
                moved.push((other, current));
            }
            let after = affected
                .iter()
                .filter_map(|index| attraction_score(&attractions[*index], &positions, &moved))
                .sum::<usize>();
            let improvement = before.saturating_sub(after);
            if after < before && improvement > best_improvement {
                best_improvement = improvement;
                best_move = Some((target, other_index, other));
            }
        }

        if let Some((target, other_index, other)) = best_move {
            person_indices[layer_index][person_index].index = target;
            positions.insert(person, target);
            if let (Some(other_index), Some(other)) = (other_index, other) {
                person_indices[layer_index][other_index].index = current;
                positions.insert(other, current);
            }
            changed = true;
        }
    }
    changed
}

fn attraction_score(
    attraction: &Attraction,
    positions: &HashMap<Pid, usize>,
    moved: &[(Pid, usize)],
) -> Option<usize> {
    let get_position = |person| {
        moved
            .iter()
            .find_map(|(moved_person, position)| (*moved_person == person).then_some(*position))
            .or_else(|| positions.get(&person).copied())
    };
    let first = get_position(attraction.people[0])?;
    let second = get_position(attraction.people[1])?;
    Some(first.abs_diff(second) * attraction.weight)
}

/// Resolve equal-score child/spouse orientations deterministically. When one
/// spouse has parents in the previous generation and the other does not, the
/// child occupies the adjacent slot nearest the parents' arithmetic center.
fn resolve_child_spouse_order(
    person_indices: &mut Grid<PersonIndex>,
    relationships: &[Relationship],
) {
    for layer_index in 1..person_indices.len() {
        let previous_positions = person_indices[layer_index - 1]
            .iter()
            .map(|person| (person.pid, person.index))
            .collect::<HashMap<_, _>>();

        for marriage in relationships
            .iter()
            .filter(|relationship| relationship.parents.iter().all(Option::is_some))
        {
            let spouses = marriage.parents.map(Option::unwrap);
            let spouse_indices = spouses.map(|spouse| {
                person_indices[layer_index]
                    .iter()
                    .position(|person| person.pid == spouse)
            });
            let [Some(first_index), Some(second_index)] = spouse_indices else {
                continue;
            };
            if person_indices[layer_index][first_index]
                .index
                .abs_diff(person_indices[layer_index][second_index].index)
                != 1
            {
                continue;
            }

            let parent_centers = spouses.map(|spouse| {
                relationships
                    .iter()
                    .filter(|relationship| relationship.children.contains(&spouse))
                    .find_map(|relationship| {
                        let positions = relationship
                            .parents
                            .iter()
                            .flatten()
                            .filter_map(|parent| previous_positions.get(parent))
                            .copied()
                            .collect_vec();
                        (!positions.is_empty())
                            .then(|| (positions.iter().sum::<usize>(), positions.len()))
                    })
            });
            let (child_index, partner_index, center_sum, center_count) = match parent_centers {
                [Some((sum, count)), None] => (first_index, second_index, sum, count),
                [None, Some((sum, count))] => (second_index, first_index, sum, count),
                _ => continue,
            };

            let child_position = person_indices[layer_index][child_index].index;
            let partner_position = person_indices[layer_index][partner_index].index;
            let child_distance = (child_position * center_count).abs_diff(center_sum);
            let partner_distance = (partner_position * center_count).abs_diff(center_sum);
            if child_distance > partner_distance
                || (child_distance == partner_distance && child_position < partner_position)
            {
                person_indices[layer_index][child_index].index = partner_position;
                person_indices[layer_index][partner_index].index = child_position;
            }
        }
    }
}

const fn middle(a: usize, b: usize) -> usize {
    let diff = b.abs_diff(a);
    if diff % 2 == 0 {
        diff / 2
    } else {
        (diff - 1) / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baumstamm_lib::{PersonId, RelationshipId};

    fn pid(value: u128) -> PersonId {
        value.into()
    }

    fn relationship(id: u128, parents: [Option<u128>; 2], children: &[u128]) -> Relationship {
        Relationship {
            id: RelationshipId::from(id),
            parents: parents.map(|parent| parent.map(pid)),
            children: children.iter().copied().map(pid).collect(),
        }
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
        let indices = get_person_indices(&layers, &[], 2, LayoutAlgorithm::Centered);

        assert_eq!(slots(&indices[0]), vec![(1, 0), (2, 1)]);
        assert_eq!(slots(&indices[1]), vec![(3, 0)]);
    }

    #[test]
    fn force_layout_keeps_generations_and_discrete_bounded_slots() {
        let layers = vec![
            vec![pid(1), pid(2), pid(3)],
            vec![pid(4), pid(5)],
            vec![pid(6)],
        ];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[4, 5]),
            relationship(2, [Some(4), Some(5)], &[6]),
        ];
        let indices =
            get_person_indices(&layers, &relationships, 3, LayoutAlgorithm::ForceDirected);

        assert_eq!(
            indices
                .iter()
                .map(|layer| layer.iter().map(|person| person.pid).collect_vec())
                .collect_vec(),
            layers
        );
        for layer in &indices {
            assert!(layer.iter().all(|person| person.index < 3));
            assert_eq!(
                layer.iter().map(|person| person.index).unique().count(),
                layer.len()
            );
        }
    }

    #[test]
    fn marriage_has_more_pull_than_siblinghood() {
        let layers = vec![vec![pid(1), pid(2), pid(3), pid(4)]];
        let relationships = vec![
            relationship(1, [None, None], &[1, 2]),
            relationship(2, [Some(1), Some(3)], &[]),
            relationship(3, [Some(1), Some(4)], &[]),
        ];
        let indices =
            get_person_indices(&layers, &relationships, 4, LayoutAlgorithm::ForceDirected);
        let positions = position_map(&indices);

        assert_eq!(positions[&pid(1)].abs_diff(positions[&pid(3)]), 1);
        assert_eq!(positions[&pid(1)].abs_diff(positions[&pid(4)]), 1);
        assert!(positions[&pid(1)].abs_diff(positions[&pid(2)]) >= 2);
    }

    #[test]
    fn child_is_ordered_inside_spouse_for_two_slot_tie() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[3]),
            relationship(2, [Some(3), Some(4)], &[]),
        ];
        let indices =
            get_person_indices(&layers, &relationships, 2, LayoutAlgorithm::ForceDirected);

        assert_eq!(slots(&indices[1]), vec![(4, 0), (3, 1)]);
    }

    #[test]
    fn child_is_moved_next_to_offset_parents_with_spouse_outside() {
        let layers = vec![vec![pid(9), pid(1), pid(2)], vec![pid(3), pid(4), pid(8)]];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[3]),
            relationship(2, [Some(3), Some(4)], &[]),
        ];
        let indices =
            get_person_indices(&layers, &relationships, 3, LayoutAlgorithm::ForceDirected);

        assert_eq!(slots(&indices[1]), vec![(4, 0), (3, 1), (8, 2)]);
    }

    #[test]
    fn force_layout_is_deterministic() {
        let layers = vec![
            vec![pid(1), pid(2), pid(3), pid(4)],
            vec![pid(5), pid(6), pid(7)],
        ];
        let relationships = vec![
            relationship(1, [Some(1), Some(4)], &[5, 6]),
            relationship(2, [Some(2), Some(3)], &[7]),
            relationship(3, [Some(5), Some(7)], &[]),
        ];
        let expected =
            get_person_indices(&layers, &relationships, 4, LayoutAlgorithm::ForceDirected);

        for _ in 0..10 {
            let actual =
                get_person_indices(&layers, &relationships, 4, LayoutAlgorithm::ForceDirected);
            assert_eq!(
                actual.iter().map(|layer| slots(layer)).collect_vec(),
                expected.iter().map(|layer| slots(layer)).collect_vec()
            );
        }
    }
}
