use super::{PersonIndexInput, PersonIndexOutput, common::project_to_slots};
use crate::indices::PersonIndex;
use baumstamm_lib::{PersonId, Relationship};
use std::collections::{BTreeMap, BTreeSet, HashMap};

type Pid = PersonId;

const ITERATIONS: usize = 360;
const ROW_WIDTH_NUMERATOR: usize = 3;
const ROW_WIDTH_DENOMINATOR: usize = 2;
const SPOUSE_WEIGHT: f64 = 16.0;
const PARENT_CHILD_WEIGHT: f64 = 9.0;
const IDENTITY_WEIGHT: f64 = 8.0;
const SIBLING_WEIGHT: f64 = 6.0;
const DISTANCE_TWO_WEIGHT: f64 = 2.0;
const FAMILY_ALIGNMENT_WEIGHT: f64 = 4.0;
const CENTERING_FORCE_SCALE: f64 = 0.012;
const ATTRACTION_FORCE_SCALE: f64 = 0.022;
const FAMILY_FORCE_SCALE: f64 = 0.035;
const MAX_ATTRACTION_FORCE: f64 = 3.0;
const INITIAL_SYMMETRY_NUDGE: f64 = 0.0001;
const REPULSION_SCALE: f64 = 0.07;
const REPULSION_SOFTENING: f64 = 0.24;
const MAX_REPULSION_FORCE: f64 = 1.1;
const COOLING_REDUCTION: f64 = 0.78;
const INTEGRATION_STEP: f64 = 0.075;
const VELOCITY_DAMPING: f64 = 0.78;
const BOUNDARY_BOUNCE_DAMPING: f64 = 0.2;
const MAX_DISCRETE_REFINEMENT_PASSES: usize = 48;

#[derive(Clone, Copy, Debug)]
struct Node {
    pid: Pid,
    layer: usize,
    order: usize,
    x: f64,
    velocity: f64,
}

#[derive(Clone, Copy, Debug)]
struct Attraction {
    people: [usize; 2],
    weight: f64,
}

#[derive(Clone, Debug)]
struct FamilyAlignment {
    parents: Vec<usize>,
    children: Vec<usize>,
}

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    let largest_layer = input
        .person_layers
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or_default();
    // ceil(largest_layer * 1.5), without floating-point rounding or overflow.
    let row_length = largest_layer
        .saturating_mul(ROW_WIDTH_NUMERATOR)
        .saturating_add(ROW_WIDTH_DENOMINATOR - 1)
        / ROW_WIDTH_DENOMINATOR;
    if row_length == 0 {
        return PersonIndexOutput {
            person_indices: Vec::new(),
            row_length,
        };
    }

    let mut nodes = initial_nodes(input.person_layers, row_length);
    let attractions = build_attractions(&nodes, &input);
    let family_alignments = build_family_alignments(&nodes, &input);
    simulate(&mut nodes, &attractions, &family_alignments, row_length);
    let mut person_indices = discretize(&nodes, input.person_layers.len(), row_length);
    refine_discrete_positions(
        &mut person_indices,
        &nodes,
        &attractions,
        &family_alignments,
        row_length,
    );

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

fn initial_nodes(person_layers: &[Vec<Pid>], row_length: usize) -> Vec<Node> {
    let center = (row_length - 1) as f64 / 2.0;
    person_layers
        .iter()
        .enumerate()
        .flat_map(|(layer, people)| {
            let span = people.len().saturating_sub(1) as f64;
            let start = center - span / 2.0;
            people.iter().enumerate().map(move |(order, pid)| Node {
                pid: *pid,
                layer,
                order,
                // Break perfectly symmetric but unstable arrangements in a
                // deterministic direction, so an unrelated person between
                // two spouses is free to move out of the pair.
                x: start + order as f64 + (order * order) as f64 * INITIAL_SYMMETRY_NUDGE,
                velocity: 0.0,
            })
        })
        .collect()
}

fn build_attractions(nodes: &[Node], input: &PersonIndexInput<'_>) -> Vec<Attraction> {
    let by_location = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ((node.layer, node.pid), index))
        .collect::<HashMap<_, _>>();
    let mut by_person = HashMap::<Pid, Vec<usize>>::new();
    for (index, node) in nodes.iter().enumerate() {
        by_person.entry(node.pid).or_default().push(index);
    }

    // A pair can be described by more than one relationship. Keeping the
    // strongest force makes the precedence explicit and avoids multiplying a
    // force merely because a person is repeated in the cut graph.
    let mut weights = BTreeMap::<(usize, usize), f64>::new();
    let mut add = |first: usize, second: usize, weight: f64| {
        if first == second {
            return;
        }
        let pair = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        weights
            .entry(pair)
            .and_modify(|old| *old = old.max(weight))
            .or_insert(weight);
    };

    // Repeated occurrences represent the same person at two necessary graph
    // layers. Pull consecutive occurrences into alignment without merging
    // them or changing either layer.
    for occurrences in by_person.values() {
        for pair in occurrences.windows(2) {
            if nodes[pair[0]].layer + 1 == nodes[pair[1]].layer {
                add(pair[0], pair[1], IDENTITY_WEIGHT);
            }
        }
    }

    let relationships = input
        .relationships
        .iter()
        .map(|relationship| (relationship.id, relationship))
        .collect::<HashMap<_, _>>();
    for (layer, relationship_ids) in input.relationship_layers.iter().enumerate() {
        for relationship_id in relationship_ids {
            let relationship = relationships
                .get(relationship_id)
                .expect("relationship layer references an unknown relationship");
            let parents = layer.checked_sub(1).map_or_else(Vec::new, |parent_layer| {
                relationship
                    .parents
                    .iter()
                    .flatten()
                    .filter_map(|pid| by_location.get(&(parent_layer, *pid)).copied())
                    .collect::<Vec<_>>()
            });
            let children = relationship
                .children
                .iter()
                .filter_map(|pid| by_location.get(&(layer, *pid)).copied())
                .collect::<Vec<_>>();

            if let [first, second] = parents.as_slice() {
                add(*first, *second, SPOUSE_WEIGHT);
            }
            for parent in &parents {
                for child in &children {
                    add(*parent, *child, PARENT_CHILD_WEIGHT);
                }
            }
            for first in 0..children.len() {
                for second in first + 1..children.len() {
                    add(children[first], children[second], SIBLING_WEIGHT);
                }
            }
        }
    }

    // Build the semantic person graph independently of the cut layers. A
    // shortest path of exactly two captures grandparents/grandchildren and
    // also uncle/aunt relationships (sibling + parent/child).
    let neighbours = relationship_neighbours(input.relationships);
    let mut distance_two = BTreeSet::new();
    for (person, direct) in &neighbours {
        for intermediate in direct {
            if let Some(second_hop) = neighbours.get(intermediate) {
                for other in second_hop {
                    if other != person && !direct.contains(other) {
                        distance_two.insert(ordered_people(*person, *other));
                    }
                }
            }
        }
    }
    for (first, second) in distance_two {
        let Some(first_occurrences) = by_person.get(&first) else {
            continue;
        };
        let Some(second_occurrences) = by_person.get(&second) else {
            continue;
        };
        for first_index in first_occurrences {
            for second_index in second_occurrences {
                if nodes[*first_index]
                    .layer
                    .abs_diff(nodes[*second_index].layer)
                    <= 2
                {
                    add(*first_index, *second_index, DISTANCE_TWO_WEIGHT);
                }
            }
        }
    }

    weights
        .into_iter()
        .map(|((first, second), weight)| Attraction {
            people: [first, second],
            weight,
        })
        .collect()
}

fn build_family_alignments(nodes: &[Node], input: &PersonIndexInput<'_>) -> Vec<FamilyAlignment> {
    let by_location = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| ((node.layer, node.pid), index))
        .collect::<HashMap<_, _>>();
    let relationships = input
        .relationships
        .iter()
        .map(|relationship| (relationship.id, relationship))
        .collect::<HashMap<_, _>>();

    input
        .relationship_layers
        .iter()
        .enumerate()
        .flat_map(|(layer, relationship_ids)| {
            let relationships = &relationships;
            let by_location = &by_location;
            relationship_ids.iter().filter_map(move |relationship_id| {
                let relationship = relationships
                    .get(relationship_id)
                    .expect("relationship layer references an unknown relationship");
                let parents = layer.checked_sub(1).map_or_else(Vec::new, |parent_layer| {
                    relationship
                        .parents
                        .iter()
                        .flatten()
                        .filter_map(|pid| by_location.get(&(parent_layer, *pid)).copied())
                        .collect::<Vec<_>>()
                });
                let children = relationship
                    .children
                    .iter()
                    .filter_map(|pid| by_location.get(&(layer, *pid)).copied())
                    .collect::<Vec<_>>();
                (!parents.is_empty() && !children.is_empty())
                    .then_some(FamilyAlignment { parents, children })
            })
        })
        .collect()
}

fn relationship_neighbours(relationships: &[Relationship]) -> BTreeMap<Pid, BTreeSet<Pid>> {
    let mut neighbours = BTreeMap::<Pid, BTreeSet<Pid>>::new();
    let mut connect = |first: Pid, second: Pid| {
        if first != second {
            neighbours.entry(first).or_default().insert(second);
            neighbours.entry(second).or_default().insert(first);
        }
    };
    for relationship in relationships {
        let parents = relationship
            .parents
            .iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        if let [first, second] = parents.as_slice() {
            connect(*first, *second);
        }
        for parent in &parents {
            for child in &relationship.children {
                connect(*parent, *child);
            }
        }
        for first in 0..relationship.children.len() {
            for second in first + 1..relationship.children.len() {
                connect(relationship.children[first], relationship.children[second]);
            }
        }
    }
    neighbours
}

fn ordered_people(first: Pid, second: Pid) -> (Pid, Pid) {
    if first < second {
        (first, second)
    } else {
        (second, first)
    }
}

fn simulate(
    nodes: &mut [Node],
    attractions: &[Attraction],
    family_alignments: &[FamilyAlignment],
    row_length: usize,
) {
    let center = (row_length - 1) as f64 / 2.0;
    let maximum = (row_length - 1) as f64;
    for iteration in 0..ITERATIONS {
        let mut forces = nodes
            .iter()
            .map(|node| (center - node.x) * CENTERING_FORCE_SCALE)
            .collect::<Vec<_>>();

        for attraction in attractions {
            let [first, second] = attraction.people;
            let delta = nodes[second].x - nodes[first].x;
            let force = (delta * attraction.weight * ATTRACTION_FORCE_SCALE)
                .clamp(-MAX_ATTRACTION_FORCE, MAX_ATTRACTION_FORCE);
            forces[first] += force;
            forces[second] -= force;
        }

        for family in family_alignments {
            let parent_center = family
                .parents
                .iter()
                .map(|index| nodes[*index].x)
                .sum::<f64>()
                / family.parents.len() as f64;
            let child_center = family
                .children
                .iter()
                .map(|index| nodes[*index].x)
                .sum::<f64>()
                / family.children.len() as f64;
            let force = ((child_center - parent_center) * FAMILY_FORCE_SCALE)
                .clamp(-MAX_ATTRACTION_FORCE, MAX_ATTRACTION_FORCE);
            for parent in &family.parents {
                forces[*parent] += force / family.parents.len() as f64;
            }
            for child in &family.children {
                forces[*child] -= force / family.children.len() as f64;
            }
        }

        // Only people sharing a layer can collide in the final grid. A soft
        // repulsion leaves enough freedom for related people to pass each
        // other during the continuous phase.
        for first in 0..nodes.len() {
            for second in first + 1..nodes.len() {
                if nodes[first].layer != nodes[second].layer {
                    continue;
                }
                let delta = nodes[second].x - nodes[first].x;
                let direction = if delta.abs() < 1e-9 {
                    if nodes[first].order < nodes[second].order {
                        1.0
                    } else {
                        -1.0
                    }
                } else {
                    delta.signum()
                };
                let repulsion = (REPULSION_SCALE / (delta.abs() + REPULSION_SOFTENING).powi(2))
                    .min(MAX_REPULSION_FORCE);
                forces[first] -= direction * repulsion;
                forces[second] += direction * repulsion;
            }
        }

        let cooling = 1.0 - iteration as f64 / ITERATIONS as f64 * COOLING_REDUCTION;
        for (node, force) in nodes.iter_mut().zip(forces) {
            node.velocity = (node.velocity + force * INTEGRATION_STEP * cooling) * VELOCITY_DAMPING;
            node.x += node.velocity;
            if node.x < 0.0 {
                node.x = 0.0;
                node.velocity *= -BOUNDARY_BOUNCE_DAMPING;
            } else if node.x > maximum {
                node.x = maximum;
                node.velocity *= -BOUNDARY_BOUNCE_DAMPING;
            }
        }
    }
}

fn discretize(nodes: &[Node], layer_count: usize, row_length: usize) -> Vec<Vec<PersonIndex>> {
    (0..layer_count)
        .map(|layer| {
            let mut layer_nodes = nodes
                .iter()
                .enumerate()
                .filter(|(_, node)| node.layer == layer)
                .collect::<Vec<_>>();
            layer_nodes.sort_by(|(_, first), (_, second)| {
                first
                    .x
                    .total_cmp(&second.x)
                    .then_with(|| first.order.cmp(&second.order))
                    .then_with(|| first.pid.cmp(&second.pid))
            });
            let targets = layer_nodes
                .iter()
                .map(|(_, node)| node.x)
                .collect::<Vec<_>>();
            let slots = project_to_slots(&targets, row_length);
            let by_order = layer_nodes
                .into_iter()
                .zip(slots)
                .map(|((_, node), index)| {
                    (
                        node.order,
                        PersonIndex {
                            index,
                            pid: node.pid,
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>();
            by_order.into_values().collect()
        })
        .collect()
}

/// Projection can collapse a sparse continuous layout into a centered block.
/// Coordinate descent restores the relationship objective on the actual grid:
/// every occurrence may move into an empty column and people may exchange
/// columns. Layers are visited in alternating directions so large early layers
/// cannot consume the entire refinement budget before descendants are seen.
fn refine_discrete_positions(
    person_indices: &mut [Vec<PersonIndex>],
    nodes: &[Node],
    attractions: &[Attraction],
    family_alignments: &[FamilyAlignment],
    row_length: usize,
) {
    let mut slots = vec![0usize; nodes.len()];
    let mut nodes_by_layer = vec![Vec::new(); person_indices.len()];
    for (node_index, node) in nodes.iter().enumerate() {
        slots[node_index] = person_indices[node.layer][node.order].index;
        nodes_by_layer[node.layer].push(node_index);
    }
    let mut incident = vec![Vec::new(); nodes.len()];
    for (attraction_index, attraction) in attractions.iter().enumerate() {
        incident[attraction.people[0]].push(attraction_index);
        incident[attraction.people[1]].push(attraction_index);
    }

    for pass in 0..MAX_DISCRETE_REFINEMENT_PASSES {
        let mut changed = false;
        let layer_order = if pass % 2 == 0 {
            (0..nodes_by_layer.len()).collect::<Vec<_>>()
        } else {
            (0..nodes_by_layer.len()).rev().collect::<Vec<_>>()
        };
        for layer in layer_order {
            let layer_nodes = &nodes_by_layer[layer];

            for node_index in layer_nodes {
                let current = slots[*node_index];
                let before = node_discrete_score(
                    &slots,
                    attractions,
                    &incident,
                    family_alignments,
                    *node_index,
                );
                let mut occupied = vec![false; row_length];
                for other in layer_nodes {
                    if other != node_index {
                        occupied[slots[*other]] = true;
                    }
                }
                let mut best = (
                    before,
                    current.abs_diff(nodes[*node_index].x.round() as usize),
                    current,
                );
                for (column, is_occupied) in occupied.iter().copied().enumerate() {
                    if is_occupied {
                        continue;
                    }
                    slots[*node_index] = column;
                    let candidate = (
                        node_discrete_score(
                            &slots,
                            attractions,
                            &incident,
                            family_alignments,
                            *node_index,
                        ),
                        column.abs_diff(nodes[*node_index].x.round() as usize),
                        column,
                    );
                    if candidate < best {
                        best = candidate;
                    }
                }
                slots[*node_index] = best.2;
                if best.0 + f64::EPSILON < before {
                    person_indices[layer][nodes[*node_index].order].index = best.2;
                    changed = true;
                } else {
                    slots[*node_index] = current;
                }
            }

            for first_offset in 0..layer_nodes.len() {
                for second_offset in first_offset + 1..layer_nodes.len() {
                    let first = layer_nodes[first_offset];
                    let second = layer_nodes[second_offset];
                    let before = affected_discrete_score(
                        &slots,
                        attractions,
                        &incident,
                        family_alignments,
                        first,
                        second,
                    );
                    slots.swap(first, second);
                    let after = affected_discrete_score(
                        &slots,
                        attractions,
                        &incident,
                        family_alignments,
                        first,
                        second,
                    );
                    if after + f64::EPSILON < before {
                        person_indices[layer][nodes[first].order].index = slots[first];
                        person_indices[layer][nodes[second].order].index = slots[second];
                        changed = true;
                    } else {
                        slots.swap(first, second);
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn node_discrete_score(
    slots: &[usize],
    attractions: &[Attraction],
    incident: &[Vec<usize>],
    family_alignments: &[FamilyAlignment],
    node: usize,
) -> f64 {
    let attraction_score = incident[node]
        .iter()
        .map(|attraction_index| {
            let attraction = attractions[*attraction_index];
            slots[attraction.people[0]].abs_diff(slots[attraction.people[1]]) as f64
                * attraction.weight
        })
        .sum::<f64>();
    let alignment_score = family_alignments
        .iter()
        .filter(|family| family.parents.contains(&node) || family.children.contains(&node))
        .map(|family| family_alignment_score(slots, family))
        .sum::<f64>();
    attraction_score + alignment_score
}

fn affected_discrete_score(
    slots: &[usize],
    attractions: &[Attraction],
    incident: &[Vec<usize>],
    family_alignments: &[FamilyAlignment],
    first: usize,
    second: usize,
) -> f64 {
    let score = |attraction_index: usize| {
        let attraction = attractions[attraction_index];
        slots[attraction.people[0]].abs_diff(slots[attraction.people[1]]) as f64 * attraction.weight
    };
    let attraction_score = incident[first]
        .iter()
        .map(|index| score(*index))
        .sum::<f64>()
        + incident[second]
            .iter()
            .filter(|index| !attractions[**index].people.contains(&first))
            .map(|index| score(*index))
            .sum::<f64>();
    let alignment_score = family_alignments
        .iter()
        .filter(|family| {
            family.parents.contains(&first)
                || family.children.contains(&first)
                || family.parents.contains(&second)
                || family.children.contains(&second)
        })
        .map(|family| family_alignment_score(slots, family))
        .sum::<f64>();
    attraction_score + alignment_score
}

fn family_alignment_score(slots: &[usize], family: &FamilyAlignment) -> f64 {
    let center_twice = |people: &[usize]| {
        let minimum = people
            .iter()
            .map(|person| slots[*person])
            .min()
            .unwrap_or_default();
        let maximum = people
            .iter()
            .map(|person| slots[*person])
            .max()
            .unwrap_or_default();
        minimum + maximum
    };
    center_twice(&family.parents).abs_diff(center_twice(&family.children)) as f64
        * FAMILY_ALIGNMENT_WEIGHT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use baumstamm_lib::RelationshipId;

    fn pid(value: u128) -> Pid {
        value.into()
    }

    fn relationship(id: u128, parents: [Option<u128>; 2], children: &[u128]) -> Relationship {
        Relationship {
            id: RelationshipId(id),
            parents: parents.map(|parent| parent.map(pid)),
            children: children.iter().copied().map(pid).collect(),
        }
    }

    fn input<'a>(
        person_layers: &'a Vec<Vec<Pid>>,
        relationship_layers: &'a Vec<Vec<RelationshipId>>,
        relationships: &'a [Relationship],
    ) -> PersonIndexInput<'a> {
        PersonIndexInput {
            person_layers,
            relationship_layers,
            relationships,
            layout_algorithm: LayoutAlgorithm::ForceDirected,
        }
    }

    fn pair_weight(nodes: &[Node], attractions: &[Attraction], first: Pid, second: Pid) -> f64 {
        attractions
            .iter()
            .filter_map(|attraction| {
                let people = attraction.people.map(|index| nodes[index].pid);
                ((people == [first, second]) || (people == [second, first]))
                    .then_some(attraction.weight)
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn force_precedence_and_distance_two_relatives_are_explicit() {
        let people = vec![
            vec![pid(1), pid(2)],
            vec![pid(3), pid(4), pid(7)],
            vec![pid(5), pid(6)],
        ];
        let relationships = vec![
            relationship(10, [Some(1), Some(2)], &[3, 4]),
            relationship(11, [Some(3), Some(7)], &[5, 6]),
        ];
        let relationship_layers = vec![vec![], vec![RelationshipId(10)], vec![RelationshipId(11)]];
        let request = input(&people, &relationship_layers, &relationships);
        let nodes = initial_nodes(&people, 5);
        let attractions = build_attractions(&nodes, &request);

        let spouse = pair_weight(&nodes, &attractions, pid(1), pid(2));
        let parent_child = pair_weight(&nodes, &attractions, pid(1), pid(3));
        let sibling = pair_weight(&nodes, &attractions, pid(3), pid(4));
        assert_eq!(spouse, SPOUSE_WEIGHT);
        assert_eq!(parent_child, PARENT_CHILD_WEIGHT);
        assert_eq!(sibling, SIBLING_WEIGHT);
        assert_eq!(
            pair_weight(&nodes, &attractions, pid(1), pid(5)),
            DISTANCE_TWO_WEIGHT
        );
        assert_eq!(
            pair_weight(&nodes, &attractions, pid(4), pid(5)),
            DISTANCE_TWO_WEIGHT
        );
        assert!(spouse > parent_child);
        assert!(parent_child > sibling);
        assert!(sibling > DISTANCE_TWO_WEIGHT);
    }

    #[test]
    fn chooses_one_and_a_half_times_the_largest_layer_and_keeps_occurrences() {
        let people = vec![vec![pid(1), pid(2), pid(3), pid(4)], vec![pid(2), pid(5)]];
        let relationship_layers = vec![vec![], vec![]];
        let output = get_person_indices(input(&people, &relationship_layers, &[]));

        assert_eq!(output.row_length, 6);
        assert_eq!(
            output
                .person_indices
                .iter()
                .map(Vec::len)
                .collect::<Vec<_>>(),
            vec![4, 2]
        );
        assert_eq!(
            output.person_indices[0]
                .iter()
                .filter(|person| person.pid == pid(2))
                .count(),
            1
        );
        assert_eq!(
            output.person_indices[1]
                .iter()
                .filter(|person| person.pid == pid(2))
                .count(),
            1
        );
        for layer in output.person_indices {
            let slots = layer
                .iter()
                .map(|person| person.index)
                .collect::<BTreeSet<_>>();
            assert_eq!(slots.len(), layer.len());
            assert!(slots.iter().all(|slot| *slot < output.row_length));
        }
    }

    #[test]
    fn simulation_is_deterministic_and_brings_spouses_together() {
        let people = vec![vec![pid(1), pid(9), pid(2)], vec![pid(3)]];
        let relationships = vec![relationship(10, [Some(1), Some(2)], &[3])];
        let relationship_layers = vec![vec![], vec![RelationshipId(10)]];

        let first = get_person_indices(input(&people, &relationship_layers, &relationships));
        let second = get_person_indices(input(&people, &relationship_layers, &relationships));
        let spouse_slots = [pid(1), pid(2)].map(|spouse| {
            first.person_indices[0]
                .iter()
                .find(|person| person.pid == spouse)
                .expect("spouse occurrence")
                .index
        });

        assert_eq!(spouse_slots[0].abs_diff(spouse_slots[1]), 1);
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
    fn projection_is_collision_free_and_minimizes_ordered_rounding_error() {
        assert_eq!(project_to_slots(&[0.2, 0.3, 3.8], 5), vec![0, 1, 4]);
        assert_eq!(project_to_slots(&[1.6, 1.7], 4), vec![1, 2]);
    }

    #[test]
    fn family_alignment_breaks_equal_length_shift_ties() {
        let family = FamilyAlignment {
            parents: vec![0, 1],
            children: vec![2, 3],
        };

        assert_eq!(family_alignment_score(&[1, 2, 1, 2], &family), 0.0);
        assert!(family_alignment_score(&[1, 2, 0, 1], &family) > 0.0);
    }
}
