use super::{PersonIndexInput, PersonIndexOutput, common::project_to_slots};
use crate::indices::PersonIndex;
use baumstamm_lib::{PersonId, Relationship};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

type Pid = PersonId;

const ITERATIONS: usize = 360;
const MIN_ITERATIONS: usize = 80;
const CONVERGENCE_VELOCITY: f64 = 0.000_05;
const CONVERGENCE_STABLE_STEPS: usize = 12;
const ROW_WIDTH_NUMERATOR: usize = 3;
const ROW_WIDTH_DENOMINATOR: usize = 2;
const IDENTITY_WEIGHT: u64 = 18;
const LEGACY_IDENTITY_WEIGHT: u64 = 8;
const SPOUSE_WEIGHT: u64 = 16;
const PARENT_CHILD_WEIGHT: u64 = 9;
const SIBLING_WEIGHT: u64 = 6;
const DISTANCE_TWO_WEIGHT: u64 = 2;
const FAMILY_ALIGNMENT_WEIGHT: u64 = 4;
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
const MAX_STRUCTURED_REFINEMENT_PASSES: usize = 8;
const MAX_BLOCK_TRANSLATION_DISTANCE: usize = 8;
const CONTINUOUS_FIDELITY_SCALE: f64 = 1_000_000.0;

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
    weight: u64,
}

#[derive(Clone, Debug)]
struct FamilyAlignment {
    parents: Vec<usize>,
    children: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttractionScope {
    /// Preserve the historical seed consumed by ConnectionOptimized.
    LegacyConnectionSeed,
    Semantic,
}

struct ForceModel {
    attractions: Vec<Attraction>,
    family_alignments: Vec<FamilyAlignment>,
    nodes_by_layer: Vec<Vec<usize>>,
    occurrences_by_person: HashMap<Pid, Vec<usize>>,
    attraction_incidence: Vec<Vec<usize>>,
    family_incidence: Vec<Vec<usize>>,
}

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    layout(input, AttractionScope::Semantic)
}

/// ConnectionOptimized historically used ForceDirected as its starting state.
/// Keep that exact seed available so improvements to the public force layout do
/// not silently alter the other algorithm's behavior.
#[allow(dead_code)]
pub(super) fn get_legacy_connection_seed(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    layout(input, AttractionScope::LegacyConnectionSeed)
}

fn layout(input: PersonIndexInput<'_>, scope: AttractionScope) -> PersonIndexOutput {
    let largest_layer = input
        .person_layers
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or_default();
    // Integer ceiling of 1.5 times the widest generation. An input large
    // enough to overflow could not be represented by the output allocation.
    let row_length = largest_layer
        .checked_mul(ROW_WIDTH_NUMERATOR)
        .and_then(|width| width.checked_add(ROW_WIDTH_DENOMINATOR - 1))
        .expect("force layout width exceeds addressable memory")
        / ROW_WIDTH_DENOMINATOR;
    if row_length == 0 {
        return PersonIndexOutput {
            person_indices: Vec::new(),
            row_length,
        };
    }

    let mut nodes = initial_nodes(input.person_layers, row_length);
    let model = build_force_model(&nodes, &input, scope);
    simulate(
        &mut nodes,
        &model.attractions,
        &model.family_alignments,
        &model.nodes_by_layer,
        row_length,
        scope,
    );
    let mut person_indices = discretize(&nodes, input.person_layers.len(), row_length);
    if scope == AttractionScope::LegacyConnectionSeed {
        refine_discrete_positions_legacy(
            &mut person_indices,
            &nodes,
            &model.attractions,
            &model.family_alignments,
            row_length,
        );
    } else {
        refine_discrete_positions(&mut person_indices, &nodes, &model, row_length);
    }

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

fn build_force_model(
    nodes: &[Node],
    input: &PersonIndexInput<'_>,
    scope: AttractionScope,
) -> ForceModel {
    let mut by_location = HashMap::with_capacity(nodes.len());
    let mut occurrences_by_person = HashMap::<Pid, Vec<usize>>::new();
    let mut nodes_by_layer = vec![Vec::new(); input.person_layers.len()];
    for (index, node) in nodes.iter().enumerate() {
        by_location.insert((node.layer, node.pid), index);
        occurrences_by_person
            .entry(node.pid)
            .or_default()
            .push(index);
        nodes_by_layer[node.layer].push(index);
    }
    let relationships = input
        .relationships
        .iter()
        .map(|relationship| (relationship.id, relationship))
        .collect::<HashMap<_, _>>();

    let attractions = build_attractions(
        nodes,
        input,
        &by_location,
        &occurrences_by_person,
        &relationships,
        scope,
    );
    let family_alignments = build_family_alignments(input, &by_location, &relationships);
    let mut attraction_incidence = vec![Vec::new(); nodes.len()];
    for (attraction_index, attraction) in attractions.iter().enumerate() {
        for person in attraction.people {
            attraction_incidence[person].push(attraction_index);
        }
    }
    let mut family_incidence = vec![Vec::new(); nodes.len()];
    for (family_index, family) in family_alignments.iter().enumerate() {
        for person in family.parents.iter().chain(&family.children) {
            family_incidence[*person].push(family_index);
        }
    }

    ForceModel {
        attractions,
        family_alignments,
        nodes_by_layer,
        occurrences_by_person,
        attraction_incidence,
        family_incidence,
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

fn build_attractions(
    nodes: &[Node],
    input: &PersonIndexInput<'_>,
    by_location: &HashMap<(usize, Pid), usize>,
    occurrences_by_person: &HashMap<Pid, Vec<usize>>,
    relationships: &HashMap<baumstamm_lib::RelationshipId, &Relationship>,
    scope: AttractionScope,
) -> Vec<Attraction> {
    // A pair can be described by more than one relationship. Keeping the
    // strongest force makes the precedence explicit and avoids multiplying a
    // force merely because a person is repeated in the cut graph.
    let mut weights = BTreeMap::<(usize, usize), u64>::new();
    let mut add = |first: usize, second: usize, weight: u64| {
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
            .and_modify(|old| *old = (*old).max(weight))
            .or_insert(weight);
    };

    // Repeated occurrences represent the same person at two necessary graph
    // layers. Pull consecutive occurrences into alignment without merging
    // them or changing either layer.
    for occurrences in occurrences_by_person.values() {
        for pair in occurrences.windows(2) {
            if scope == AttractionScope::Semantic
                || nodes[pair[0]].layer.checked_add(1) == Some(nodes[pair[1]].layer)
            {
                let weight = if scope == AttractionScope::Semantic {
                    IDENTITY_WEIGHT
                } else {
                    LEGACY_IDENTITY_WEIGHT
                };
                add(pair[0], pair[1], weight);
            }
        }
    }

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
        let Some(first_occurrences) = occurrences_by_person.get(&first) else {
            continue;
        };
        let Some(second_occurrences) = occurrences_by_person.get(&second) else {
            continue;
        };
        for first_index in first_occurrences {
            for second_index in second_occurrences {
                if scope == AttractionScope::Semantic
                    || nodes[*first_index]
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

fn build_family_alignments(
    input: &PersonIndexInput<'_>,
    by_location: &HashMap<(usize, Pid), usize>,
    relationships: &HashMap<baumstamm_lib::RelationshipId, &Relationship>,
) -> Vec<FamilyAlignment> {
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
    nodes_by_layer: &[Vec<usize>],
    row_length: usize,
    scope: AttractionScope,
) {
    let center = (row_length - 1) as f64 / 2.0;
    let maximum = (row_length - 1) as f64;
    let mut stable_steps = 0;
    for iteration in 0..ITERATIONS {
        let mut forces = nodes
            .iter()
            .map(|node| (center - node.x) * CENTERING_FORCE_SCALE)
            .collect::<Vec<_>>();

        for attraction in attractions {
            let [first, second] = attraction.people;
            let delta = nodes[second].x - nodes[first].x;
            let force = (delta * attraction.weight as f64 * ATTRACTION_FORCE_SCALE)
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
        if scope == AttractionScope::LegacyConnectionSeed {
            for first in 0..nodes.len() {
                for second in first + 1..nodes.len() {
                    if nodes[first].layer != nodes[second].layer {
                        continue;
                    }
                    add_repulsion(nodes, &mut forces, first, second);
                }
            }
        } else {
            for layer_nodes in nodes_by_layer {
                for first_offset in 0..layer_nodes.len() {
                    for second_offset in first_offset + 1..layer_nodes.len() {
                        let first = layer_nodes[first_offset];
                        let second = layer_nodes[second_offset];
                        add_repulsion(nodes, &mut forces, first, second);
                    }
                }
            }
        }

        let cooling = 1.0 - iteration as f64 / ITERATIONS as f64 * COOLING_REDUCTION;
        let mut maximum_velocity = 0.0_f64;
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
            maximum_velocity = maximum_velocity.max(node.velocity.abs());
        }
        if scope == AttractionScope::Semantic && iteration + 1 >= MIN_ITERATIONS {
            if maximum_velocity <= CONVERGENCE_VELOCITY {
                stable_steps += 1;
                if stable_steps >= CONVERGENCE_STABLE_STEPS {
                    break;
                }
            } else {
                stable_steps = 0;
            }
        }
    }
}

fn add_repulsion(nodes: &[Node], forces: &mut [f64], first: usize, second: usize) {
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
    let repulsion =
        (REPULSION_SCALE / (delta.abs() + REPULSION_SOFTENING).powi(2)).min(MAX_REPULSION_FORCE);
    forces[first] -= direction * repulsion;
    forces[second] += direction * repulsion;
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

#[derive(Clone, Debug)]
struct DiscreteScore {
    relationship_energy: u128,
    family_alignment: u128,
    used_span: u128,
    center_offset: u128,
    continuous_fidelity: u128,
}

impl DiscreteScore {
    fn cmp(&self, other: &Self) -> Ordering {
        self.relationship_energy
            .cmp(&other.relationship_energy)
            .then_with(|| self.family_alignment.cmp(&other.family_alignment))
            .then_with(|| self.used_span.cmp(&other.used_span))
            .then_with(|| self.center_offset.cmp(&other.center_offset))
            .then_with(|| self.continuous_fidelity.cmp(&other.continuous_fidelity))
    }
}

/// Refine the projected grid with one strict, deterministic lexicographic
/// objective. Relationship energy leads, followed by family-center alignment,
/// compact occupied spans, centering, fidelity to the continuous simulation,
/// and finally canonical coordinates. Every accepted operation strictly
/// decreases this key, so neutral staging cannot cycle.
fn refine_discrete_positions(
    person_indices: &mut [Vec<PersonIndex>],
    nodes: &[Node],
    model: &ForceModel,
    row_length: usize,
) {
    let mut slots = vec![0usize; nodes.len()];
    let mut occupancy = vec![vec![None; row_length]; model.nodes_by_layer.len()];
    for (node_index, node) in nodes.iter().enumerate() {
        let column = person_indices[node.layer][node.order].index;
        slots[node_index] = column;
        assert!(
            occupancy[node.layer][column].replace(node_index).is_none(),
            "discretization must be collision-free"
        );
    }

    let blocks = structured_blocks(nodes, model);
    let mut current_score = discrete_score(&slots, nodes, model, row_length);
    for pass in 0..MAX_STRUCTURED_REFINEMENT_PASSES {
        let mut changed = false;

        // A direct block translation can cross a local barrier that no
        // relationship-improving single occurrence move can cross.
        for block in &blocks {
            for distance in 1..row_length.min(MAX_BLOCK_TRANSLATION_DISTANCE + 1) {
                for delta in [-(distance as i128), distance as i128] {
                    let Some(changes) = translated_changes(block, delta, &slots, row_length) else {
                        continue;
                    };
                    changed |= consider_changes(
                        &changes,
                        &mut slots,
                        &mut occupancy,
                        nodes,
                        model,
                        row_length,
                        &mut current_score,
                    );
                }
            }
        }

        // Exchange the centers of two disjoint coherent units in one move.
        // Trying both integer roundings supports half-column family centers.
        for first in 0..blocks.len() {
            for second in first + 1..blocks.len() {
                if blocks[first]
                    .iter()
                    .any(|node| blocks[second].binary_search(node).is_ok())
                {
                    continue;
                }
                let first_center = block_center_twice(&blocks[first], &slots);
                let second_center = block_center_twice(&blocks[second], &slots);
                let difference = second_center - first_center;
                let lower = difference.div_euclid(2);
                let upper = lower + difference.rem_euclid(2);
                for first_delta in [lower, upper] {
                    if first_delta == 0 {
                        continue;
                    }
                    let Some(mut changes) =
                        translated_changes(&blocks[first], first_delta, &slots, row_length)
                    else {
                        continue;
                    };
                    let Some(second_changes) =
                        translated_changes(&blocks[second], -first_delta, &slots, row_length)
                    else {
                        continue;
                    };
                    changes.extend(second_changes);
                    changed |= consider_changes(
                        &changes,
                        &mut slots,
                        &mut occupancy,
                        nodes,
                        model,
                        row_length,
                        &mut current_score,
                    );
                }
            }
        }

        let layers: Box<dyn Iterator<Item = usize>> = if pass % 2 == 0 {
            Box::new(0..model.nodes_by_layer.len())
        } else {
            Box::new((0..model.nodes_by_layer.len()).rev())
        };
        for layer in layers {
            for &node in &model.nodes_by_layer[layer] {
                for column in 0..row_length {
                    if occupancy[layer][column].is_none() {
                        changed |= consider_changes(
                            &[(node, column)],
                            &mut slots,
                            &mut occupancy,
                            nodes,
                            model,
                            row_length,
                            &mut current_score,
                        );
                    }
                }
            }
            for first in 0..model.nodes_by_layer[layer].len() {
                for second in first + 1..model.nodes_by_layer[layer].len() {
                    let first_node = model.nodes_by_layer[layer][first];
                    let second_node = model.nodes_by_layer[layer][second];
                    changed |= consider_changes(
                        &[
                            (first_node, slots[second_node]),
                            (second_node, slots[first_node]),
                        ],
                        &mut slots,
                        &mut occupancy,
                        nodes,
                        model,
                        row_length,
                        &mut current_score,
                    );
                }
            }
        }

        if !changed {
            break;
        }
    }

    for (node_index, node) in nodes.iter().enumerate() {
        person_indices[node.layer][node.order].index = slots[node_index];
    }
}

fn discrete_score(
    slots: &[usize],
    nodes: &[Node],
    model: &ForceModel,
    row_length: usize,
) -> DiscreteScore {
    let relationship_energy = model
        .attractions
        .iter()
        .map(|attraction| relationship_energy(slots, attraction))
        .sum();
    let family_alignment = model
        .family_alignments
        .iter()
        .map(|family| family_alignment_energy(slots, family))
        .sum();
    let mut used_span = 0_u128;
    let mut center_offset = 0_u128;
    for layer in &model.nodes_by_layer {
        let (layer_span, layer_offset) = layer_compactness(layer, slots, row_length);
        used_span += layer_span;
        center_offset += layer_offset;
    }
    let continuous_fidelity = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| continuous_fidelity(slots[index], node))
        .sum();

    DiscreteScore {
        relationship_energy,
        family_alignment,
        used_span,
        center_offset,
        continuous_fidelity,
    }
}

fn relationship_energy(slots: &[usize], attraction: &Attraction) -> u128 {
    slots[attraction.people[0]].abs_diff(slots[attraction.people[1]]) as u128
        * u128::from(attraction.weight)
}

fn family_alignment_energy(slots: &[usize], family: &FamilyAlignment) -> u128 {
    family_center_twice(&family.parents, slots)
        .abs_diff(family_center_twice(&family.children, slots))
        * u128::from(FAMILY_ALIGNMENT_WEIGHT)
}

fn layer_compactness(layer: &[usize], slots: &[usize], row_length: usize) -> (u128, u128) {
    let Some(minimum) = layer.iter().map(|node| slots[*node]).min() else {
        return (0, 0);
    };
    let maximum = layer
        .iter()
        .map(|node| slots[*node])
        .max()
        .expect("non-empty layer");
    let center_twice = minimum as u128 + maximum as u128;
    (
        maximum.abs_diff(minimum) as u128,
        center_twice.abs_diff(row_length.saturating_sub(1) as u128),
    )
}

fn continuous_fidelity(column: usize, node: &Node) -> u128 {
    ((column as f64 - node.x).powi(2) * CONTINUOUS_FIDELITY_SCALE).round() as u128
}

#[allow(clippy::too_many_arguments)]
fn consider_changes(
    changes: &[(usize, usize)],
    slots: &mut [usize],
    occupancy: &mut [Vec<Option<usize>>],
    nodes: &[Node],
    model: &ForceModel,
    row_length: usize,
    current_score: &mut DiscreteScore,
) -> bool {
    let mut moved = BTreeSet::new();
    let mut targets = BTreeSet::new();
    for &(node, column) in changes {
        if column >= row_length || !moved.insert(node) {
            return false;
        }
        let layer = nodes[node].layer;
        if !targets.insert((layer, column)) {
            return false;
        }
    }
    if changes.iter().all(|(node, column)| slots[*node] == *column) {
        return false;
    }
    for &(node, column) in changes {
        if occupancy[nodes[node].layer][column].is_some_and(|other| !moved.contains(&other)) {
            return false;
        }
    }

    let affected_attractions = moved
        .iter()
        .flat_map(|node| model.attraction_incidence[*node].iter().copied())
        .collect::<BTreeSet<_>>();
    let affected_families = moved
        .iter()
        .flat_map(|node| model.family_incidence[*node].iter().copied())
        .collect::<BTreeSet<_>>();
    let affected_layers = moved
        .iter()
        .map(|node| nodes[*node].layer)
        .collect::<BTreeSet<_>>();
    let old_relationship_energy = affected_attractions
        .iter()
        .map(|index| relationship_energy(slots, &model.attractions[*index]))
        .sum::<u128>();
    let old_family_alignment = affected_families
        .iter()
        .map(|index| family_alignment_energy(slots, &model.family_alignments[*index]))
        .sum::<u128>();
    let (old_used_span, old_center_offset) = affected_layers
        .iter()
        .map(|layer| layer_compactness(&model.nodes_by_layer[*layer], slots, row_length))
        .fold((0_u128, 0_u128), |(span, offset), candidate| {
            (span + candidate.0, offset + candidate.1)
        });
    let old_continuous_fidelity = moved
        .iter()
        .map(|node| continuous_fidelity(slots[*node], &nodes[*node]))
        .sum::<u128>();
    let previous = changes
        .iter()
        .map(|(node, _)| (*node, slots[*node]))
        .collect::<Vec<_>>();
    for &(node, _) in changes {
        occupancy[nodes[node].layer][slots[node]] = None;
    }
    for &(node, column) in changes {
        slots[node] = column;
        occupancy[nodes[node].layer][column] = Some(node);
    }

    let new_relationship_energy = affected_attractions
        .iter()
        .map(|index| relationship_energy(slots, &model.attractions[*index]))
        .sum::<u128>();
    let new_family_alignment = affected_families
        .iter()
        .map(|index| family_alignment_energy(slots, &model.family_alignments[*index]))
        .sum::<u128>();
    let (new_used_span, new_center_offset) = affected_layers
        .iter()
        .map(|layer| layer_compactness(&model.nodes_by_layer[*layer], slots, row_length))
        .fold((0_u128, 0_u128), |(span, offset), candidate| {
            (span + candidate.0, offset + candidate.1)
        });
    let new_continuous_fidelity = moved
        .iter()
        .map(|node| continuous_fidelity(slots[*node], &nodes[*node]))
        .sum::<u128>();
    let candidate = DiscreteScore {
        relationship_energy: current_score.relationship_energy - old_relationship_energy
            + new_relationship_energy,
        family_alignment: current_score.family_alignment - old_family_alignment
            + new_family_alignment,
        used_span: current_score.used_span - old_used_span + new_used_span,
        center_offset: current_score.center_offset - old_center_offset + new_center_offset,
        continuous_fidelity: current_score.continuous_fidelity - old_continuous_fidelity
            + new_continuous_fidelity,
    };
    let score_ordering = candidate.cmp(current_score);
    let canonical_ordering = || {
        slots
            .iter()
            .enumerate()
            .find_map(|(node, candidate_column)| {
                let previous_column = previous
                    .iter()
                    .find_map(|(changed_node, column)| (*changed_node == node).then_some(column))
                    .unwrap_or(candidate_column);
                (candidate_column != previous_column).then(|| candidate_column.cmp(previous_column))
            })
            .unwrap_or(Ordering::Equal)
    };
    if score_ordering == Ordering::Less
        || (score_ordering == Ordering::Equal && canonical_ordering() == Ordering::Less)
    {
        *current_score = candidate;
        true
    } else {
        for &(node, _) in changes {
            occupancy[nodes[node].layer][slots[node]] = None;
        }
        for (node, column) in previous {
            slots[node] = column;
            occupancy[nodes[node].layer][column] = Some(node);
        }
        false
    }
}

fn translated_changes(
    block: &[usize],
    delta: i128,
    slots: &[usize],
    row_length: usize,
) -> Option<Vec<(usize, usize)>> {
    block
        .iter()
        .map(|node| {
            let shifted = i128::try_from(slots[*node]).ok()?.checked_add(delta)?;
            let column = usize::try_from(shifted).ok()?;
            (column < row_length).then_some((*node, column))
        })
        .collect()
}

fn structured_blocks(nodes: &[Node], model: &ForceModel) -> Vec<Vec<usize>> {
    let mut parent_families = vec![Vec::new(); nodes.len()];
    for (family_index, family) in model.family_alignments.iter().enumerate() {
        for parent in &family.parents {
            parent_families[*parent].push(family_index);
        }
    }

    let mut unique = BTreeSet::new();
    for family in &model.family_alignments {
        let direct = family
            .parents
            .iter()
            .chain(&family.children)
            .copied()
            .collect::<BTreeSet<_>>();
        unique.insert(direct.into_iter().collect::<Vec<_>>());

        let mut subtree = BTreeSet::new();
        let mut queue = family
            .parents
            .iter()
            .chain(&family.children)
            .copied()
            .collect::<VecDeque<_>>();
        while let Some(node) = queue.pop_front() {
            if !subtree.insert(node) {
                continue;
            }
            if let Some(occurrences) = model.occurrences_by_person.get(&nodes[node].pid) {
                queue.extend(occurrences.iter().copied());
            }
            for family_index in &parent_families[node] {
                let descendant_family = &model.family_alignments[*family_index];
                queue.extend(descendant_family.parents.iter().copied());
                queue.extend(descendant_family.children.iter().copied());
            }
        }
        unique.insert(subtree.into_iter().collect::<Vec<_>>());
    }
    unique.into_iter().filter(|block| block.len() > 1).collect()
}

fn family_center_twice(people: &[usize], slots: &[usize]) -> u128 {
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
    minimum as u128 + maximum as u128
}

fn block_center_twice(block: &[usize], slots: &[usize]) -> i128 {
    let minimum = block
        .iter()
        .map(|node| slots[*node])
        .min()
        .unwrap_or_default();
    let maximum = block
        .iter()
        .map(|node| slots[*node])
        .max()
        .unwrap_or_default();
    i128::try_from(minimum)
        .ok()
        .and_then(|minimum| {
            i128::try_from(maximum)
                .ok()
                .and_then(|maximum| minimum.checked_add(maximum))
        })
        .expect("grid columns fit in i128")
}

/// Projection can collapse a sparse continuous layout into a centered block.
/// Coordinate descent restores the relationship objective on the actual grid:
/// every occurrence may move into an empty column and people may exchange
/// columns. Layers are visited in alternating directions so large early layers
/// cannot consume the entire refinement budget before descendants are seen.
fn refine_discrete_positions_legacy(
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
                * attraction.weight as f64
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
        slots[attraction.people[0]].abs_diff(slots[attraction.people[1]]) as f64
            * attraction.weight as f64
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
        * FAMILY_ALIGNMENT_WEIGHT as f64
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

    fn pair_weight(nodes: &[Node], attractions: &[Attraction], first: Pid, second: Pid) -> u64 {
        attractions
            .iter()
            .filter_map(|attraction| {
                let people = attraction.people.map(|index| nodes[index].pid);
                ((people == [first, second]) || (people == [second, first]))
                    .then_some(attraction.weight)
            })
            .max()
            .unwrap_or_default()
    }

    fn slot(output: &PersonIndexOutput, layer: usize, person: Pid) -> usize {
        output.person_indices[layer]
            .iter()
            .find(|occurrence| occurrence.pid == person)
            .expect("person occurrence")
            .index
    }

    fn center_twice(output: &PersonIndexOutput, layer: usize, people: &[u128]) -> usize {
        let columns = people
            .iter()
            .map(|person| slot(output, layer, pid(*person)))
            .collect::<Vec<_>>();
        columns.iter().min().expect("non-empty family")
            + columns.iter().max().expect("non-empty family")
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
        let model = build_force_model(&nodes, &request, AttractionScope::Semantic);
        let attractions = model.attractions;

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
    fn semantic_distance_two_force_ignores_incidental_layer_distance() {
        let people = vec![vec![pid(1)], vec![pid(3)], vec![pid(9)], vec![pid(5)]];
        let relationships = vec![
            relationship(10, [Some(1), None], &[3]),
            relationship(11, [Some(3), None], &[5]),
        ];
        let relationship_layers = vec![vec![], vec![], vec![], vec![]];
        let request = input(&people, &relationship_layers, &relationships);
        let nodes = initial_nodes(&people, 2);
        let model = build_force_model(&nodes, &request, AttractionScope::Semantic);

        assert_eq!(
            pair_weight(&nodes, &model.attractions, pid(1), pid(5)),
            DISTANCE_TWO_WEIGHT
        );
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
    fn odd_widest_generation_uses_ceiling_and_compacts_the_used_span() {
        let people = vec![vec![pid(1), pid(2), pid(3), pid(4), pid(5)], vec![pid(6)]];
        let relationship_layers = vec![vec![], vec![]];
        let output = get_person_indices(input(&people, &relationship_layers, &[]));
        let columns = output.person_indices[0]
            .iter()
            .map(|person| person.index)
            .collect::<Vec<_>>();
        let minimum = *columns.iter().min().expect("non-empty generation");
        let maximum = *columns.iter().max().expect("non-empty generation");

        assert_eq!(output.row_length, 8);
        assert_eq!(maximum - minimum, people[0].len() - 1);
        assert!((minimum + maximum).abs_diff(output.row_length - 1) <= 1);
    }

    #[test]
    fn isolated_family_is_centered_and_shares_one_center() {
        let people = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![relationship(10, [Some(1), Some(2)], &[3, 4])];
        let relationship_layers = vec![vec![], vec![RelationshipId(10)]];
        let output = get_person_indices(input(&people, &relationship_layers, &relationships));
        let parent_center = center_twice(&output, 0, &[1, 2]);
        let child_center = center_twice(&output, 1, &[3, 4]);

        assert_eq!(parent_center, child_center);
        assert!(parent_center.abs_diff(output.row_length - 1) <= 1);
    }

    #[test]
    fn repeated_occurrences_align_across_generation_gaps() {
        let people = vec![
            vec![pid(1), pid(2)],
            vec![pid(3), pid(4)],
            vec![pid(5), pid(1)],
        ];
        let relationship_layers = vec![vec![], vec![], vec![]];
        let output = get_person_indices(input(&people, &relationship_layers, &[]));

        assert_eq!(slot(&output, 0, pid(1)), slot(&output, 2, pid(1)));
    }

    #[test]
    fn family_subtrees_can_reorder_across_multiple_layers() {
        let people = vec![
            vec![pid(1), pid(2), pid(3), pid(4)],
            vec![pid(7), pid(8), pid(5), pid(6)],
            vec![pid(11), pid(12), pid(9), pid(10)],
        ];
        let relationships = vec![
            relationship(10, [Some(1), Some(2)], &[5, 6]),
            relationship(11, [Some(3), Some(4)], &[7, 8]),
            relationship(12, [Some(5), None], &[9, 10]),
            relationship(13, [Some(7), None], &[11, 12]),
        ];
        let relationship_layers = vec![
            vec![],
            vec![RelationshipId(10), RelationshipId(11)],
            vec![RelationshipId(12), RelationshipId(13)],
        ];
        let output = get_person_indices(input(&people, &relationship_layers, &relationships));

        let root_order = center_twice(&output, 0, &[1, 2]).cmp(&center_twice(&output, 0, &[3, 4]));
        assert_ne!(root_order, Ordering::Equal);
        assert_eq!(
            center_twice(&output, 1, &[5, 6]).cmp(&center_twice(&output, 1, &[7, 8])),
            root_order
        );
        assert_eq!(
            center_twice(&output, 2, &[9, 10]).cmp(&center_twice(&output, 2, &[11, 12])),
            root_order
        );
        // Both descendant layers began in the opposite order.
        assert_eq!(people[1][0], pid(7));
        assert_eq!(people[2][0], pid(11));
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
