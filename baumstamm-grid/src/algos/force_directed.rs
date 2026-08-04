use super::{PersonIndexInput, centered};
use crate::{Grid, indices::PersonIndex};
use baumstamm_lib::Relationship;
use itertools::Itertools;
use std::collections::HashMap;

type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct PersonLocation {
    layer: usize,
    pid: Pid,
}

#[derive(Clone, Copy)]
struct Attraction {
    people: [PersonLocation; 2],
    weight: usize,
}

const MARRIAGE_WEIGHT: usize = 12;
const PARENT_CHILD_WEIGHT: usize = 5;
const SIBLING_WEIGHT: usize = 3;
// The objective contains minimum linear arrangement as a special case, so an
// exact solver is necessarily exponential in the width of a generation.
// Dynamic programming keeps it practical when each generation has a manageable
// number of arrangements, even if their Cartesian product is enormous.
const EXACT_TOTAL_STATE_LIMIT: usize = 250_000;
const EXACT_TRANSITION_LIMIT: usize = 10_000_000;
const MAX_SWAP_PASSES: usize = 32;
const MAX_TRANSLATION_WIDTH: usize = 64;

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> Grid<PersonIndex> {
    let attractions = attractions(input.relationship_layers, input.relationships);
    let mut positions = exact_layered_layout(input.person_layers, input.row_length, &attractions)
        .unwrap_or_else(|| heuristic_layout(input, &attractions));

    resolve_child_spouse_order(&mut positions, input.relationships, &attractions);
    positions
}

#[derive(Clone, Copy)]
struct InternalEdge {
    people: [usize; 2],
    weight: usize,
}

#[derive(Clone, Copy)]
struct CrossLayerEdge {
    previous_person: usize,
    current_person: usize,
    weight: usize,
}

fn exact_layered_layout(
    person_layers: &Grid<Pid>,
    row_length: usize,
    attractions: &[Attraction],
) -> Option<Grid<PersonIndex>> {
    let state_counts = person_layers
        .iter()
        .map(|layer| permutation_count(row_length, layer.len()))
        .collect::<Option<Vec<_>>>()?;
    let total_states = state_counts
        .iter()
        .try_fold(0usize, |total, states| total.checked_add(*states))?;
    let transitions = state_counts.windows(2).try_fold(0usize, |total, pair| {
        total.checked_add(pair[0].checked_mul(pair[1])?)
    })?;
    if total_states > EXACT_TOTAL_STATE_LIMIT || transitions > EXACT_TRANSITION_LIMIT {
        return None;
    }

    let person_indices_by_layer = person_layers
        .iter()
        .map(|layer| {
            layer
                .iter()
                .enumerate()
                .map(|(index, person)| (*person, index))
                .collect::<HashMap<_, _>>()
        })
        .collect_vec();
    let mut internal_edges = vec![Vec::<InternalEdge>::new(); person_layers.len()];
    let mut cross_edges = vec![Vec::<CrossLayerEdge>::new(); person_layers.len()];
    for attraction in attractions {
        let [first, second] = attraction.people;
        if first.layer == second.layer {
            internal_edges[first.layer].push(InternalEdge {
                people: [
                    person_indices_by_layer[first.layer][&first.pid],
                    person_indices_by_layer[first.layer][&second.pid],
                ],
                weight: attraction.weight,
            });
            continue;
        }

        let (previous, current) = if first.layer < second.layer {
            (first, second)
        } else {
            (second, first)
        };
        if current.layer != previous.layer + 1 {
            return None;
        }
        cross_edges[current.layer].push(CrossLayerEdge {
            previous_person: person_indices_by_layer[previous.layer][&previous.pid],
            current_person: person_indices_by_layer[current.layer][&current.pid],
            weight: attraction.weight,
        });
    }

    let states = person_layers
        .iter()
        .map(|layer| {
            let mut states = Vec::new();
            let mut positions = vec![0; layer.len()];
            let mut occupied = vec![false; row_length];
            enumerate_layer_states(0, &mut positions, &mut occupied, &mut states);
            states
        })
        .collect_vec();

    let mut predecessors = states
        .iter()
        .map(|layer_states| vec![usize::MAX; layer_states.len()])
        .collect_vec();
    let mut previous_scores = states[0]
        .iter()
        .map(|state| internal_edge_score(state, &internal_edges[0]))
        .collect_vec();

    for layer_index in 1..states.len() {
        let mut current_scores = vec![usize::MAX; states[layer_index].len()];
        for (current_index, current_state) in states[layer_index].iter().enumerate() {
            let internal_score = internal_edge_score(current_state, &internal_edges[layer_index]);
            for (previous_index, previous_state) in states[layer_index - 1].iter().enumerate() {
                let score = previous_scores[previous_index]
                    + internal_score
                    + cross_layer_edge_score(
                        previous_state,
                        current_state,
                        &cross_edges[layer_index],
                    );
                if score < current_scores[current_index] {
                    current_scores[current_index] = score;
                    predecessors[layer_index][current_index] = previous_index;
                }
            }
        }
        previous_scores = current_scores;
    }

    let mut state_index = previous_scores
        .iter()
        .enumerate()
        .min_by_key(|(_, score)| *score)
        .map(|(index, _)| index)?;
    let mut selected_states = vec![0; states.len()];
    for layer_index in (0..states.len()).rev() {
        selected_states[layer_index] = state_index;
        if layer_index > 0 {
            state_index = predecessors[layer_index][state_index];
        }
    }

    Some(
        person_layers
            .iter()
            .enumerate()
            .map(|(layer_index, people)| {
                people
                    .iter()
                    .zip(&states[layer_index][selected_states[layer_index]])
                    .map(|(person, position)| PersonIndex {
                        index: *position,
                        pid: *person,
                    })
                    .collect_vec()
            })
            .collect_vec(),
    )
}

fn permutation_count(slots: usize, people: usize) -> Option<usize> {
    (0..people).try_fold(1usize, |count, offset| {
        count.checked_mul(slots.checked_sub(offset)?)
    })
}

fn enumerate_layer_states(
    next_person: usize,
    positions: &mut [usize],
    occupied: &mut [bool],
    states: &mut Vec<Vec<usize>>,
) {
    if next_person == positions.len() {
        states.push(positions.to_vec());
        return;
    }

    for slot in 0..occupied.len() {
        if occupied[slot] {
            continue;
        }
        occupied[slot] = true;
        positions[next_person] = slot;
        enumerate_layer_states(next_person + 1, positions, occupied, states);
        occupied[slot] = false;
    }
}

fn internal_edge_score(state: &[usize], edges: &[InternalEdge]) -> usize {
    edges
        .iter()
        .map(|edge| state[edge.people[0]].abs_diff(state[edge.people[1]]) * edge.weight)
        .sum()
}

fn cross_layer_edge_score(
    previous_state: &[usize],
    current_state: &[usize],
    edges: &[CrossLayerEdge],
) -> usize {
    edges
        .iter()
        .map(|edge| {
            previous_state[edge.previous_person].abs_diff(current_state[edge.current_person])
                * edge.weight
        })
        .sum()
}

const BARYCENTRIC_SWEEPS: usize = 12;

fn barycentric_layout(
    input: PersonIndexInput<'_>,
    attractions: &[Attraction],
) -> Grid<PersonIndex> {
    let row_length = input.row_length;
    let mut positions = centered::get_person_indices(input);
    let mut best = positions.clone();
    let mut best_score = layout_score(&best, attractions);
    let mut neighbours = HashMap::<PersonLocation, Vec<(PersonLocation, usize)>>::new();
    for attraction in attractions {
        let [first, second] = attraction.people;
        neighbours
            .entry(first)
            .or_default()
            .push((second, attraction.weight));
        neighbours
            .entry(second)
            .or_default()
            .push((first, attraction.weight));
    }

    let mut position_lookup = position_map(&positions);
    for sweep in 0..BARYCENTRIC_SWEEPS {
        let layer_indices = if sweep % 2 == 0 {
            (0..positions.len()).collect_vec()
        } else {
            (0..positions.len()).rev().collect_vec()
        };
        for layer_index in layer_indices {
            let mut targets = positions[layer_index]
                .iter()
                .enumerate()
                .map(|(person_index, person)| {
                    let location = PersonLocation {
                        layer: layer_index,
                        pid: person.pid,
                    };
                    let incident = neighbours.get(&location).map(Vec::as_slice).unwrap_or(&[]);
                    let total_weight = incident.iter().map(|(_, weight)| *weight).sum::<usize>();
                    let neighbour_sum = incident
                        .iter()
                        .map(|(other, weight)| position_lookup[other] * weight)
                        .sum::<usize>();
                    let target = if total_weight == 0 {
                        person.index as f64
                    } else {
                        // Equal self-weight damps spouse-order oscillation while
                        // preserving the neighbours' weighted barycenter.
                        (person.index * total_weight + neighbour_sum) as f64
                            / (2 * total_weight) as f64
                    };
                    (person_index, target, person.index, person.pid)
                })
                .collect_vec();
            // Targets are finite and non-negative, so their IEEE bit pattern
            // has the same order as the numeric value.
            radix_sort_by_u64(&mut targets, |target| target.1.to_bits());
            let projected = project_distinct_positions(
                &targets.iter().map(|target| target.1).collect_vec(),
                row_length,
            );
            for ((person_index, _, _, _), position) in targets.into_iter().zip(projected) {
                positions[layer_index][person_index].index = position;
                position_lookup.insert(
                    PersonLocation {
                        layer: layer_index,
                        pid: positions[layer_index][person_index].pid,
                    },
                    position,
                );
            }
        }

        let score = layout_score(&positions, attractions);
        if score < best_score {
            best_score = score;
            best = positions.clone();
        }
    }
    best
}

#[derive(Clone, Copy)]
struct ProjectionBlock {
    start: usize,
    end: usize,
    sum: f64,
}

impl ProjectionBlock {
    fn mean(self) -> f64 {
        self.sum / (self.end - self.start) as f64
    }
}

fn radix_sort_by_u64<T: Copy>(values: &mut Vec<T>, key: impl Fn(&T) -> u64) {
    if values.len() < 2 {
        return;
    }
    let mut buffer = values.clone();
    for byte_index in 0..8 {
        let shift = byte_index * 8;
        let mut counts = [0usize; 256];
        for value in values.iter() {
            counts[((key(value) >> shift) & 0xff) as usize] += 1;
        }
        let mut offsets = [0usize; 256];
        for index in 1..offsets.len() {
            offsets[index] = offsets[index - 1] + counts[index - 1];
        }
        for value in values.iter() {
            let bucket = ((key(value) >> shift) & 0xff) as usize;
            buffer[offsets[bucket]] = *value;
            offsets[bucket] += 1;
        }
        std::mem::swap(values, &mut buffer);
    }
}

/// Project sorted continuous targets onto distinct integer slots in linear
/// time. Subtracting each person's rank turns strict slot ordering into
/// ordinary isotonic regression.
fn project_distinct_positions(targets: &[f64], row_length: usize) -> Vec<usize> {
    let mut blocks = Vec::<ProjectionBlock>::with_capacity(targets.len());
    for (index, target) in targets.iter().enumerate() {
        blocks.push(ProjectionBlock {
            start: index,
            end: index + 1,
            sum: target - index as f64,
        });
        while blocks.len() >= 2 && blocks[blocks.len() - 2].mean() > blocks[blocks.len() - 1].mean()
        {
            let right = blocks.pop().expect("right projection block");
            let left = blocks.pop().expect("left projection block");
            blocks.push(ProjectionBlock {
                start: left.start,
                end: right.end,
                sum: left.sum + right.sum,
            });
        }
    }

    let maximum_offset = row_length - targets.len();
    let mut positions = vec![0; targets.len()];
    for block in blocks {
        let offset = block.mean().round().clamp(0.0, maximum_offset as f64) as usize;
        for (index, position) in positions
            .iter_mut()
            .enumerate()
            .take(block.end)
            .skip(block.start)
        {
            *position = offset + index;
        }
    }
    positions
}

fn heuristic_layout(input: PersonIndexInput<'_>, attractions: &[Attraction]) -> Grid<PersonIndex> {
    let row_length = input.row_length;
    let mut positions = barycentric_layout(input, attractions);
    let incident_attractions = incident_attractions(attractions);

    if row_length > 24 {
        for _ in 0..8 {
            refine_targeted_moves(
                &mut positions,
                row_length,
                attractions,
                &incident_attractions,
            );
            refine_adjacent_people(&mut positions, attractions, &incident_attractions);
            relax_ordered_coordinates(&mut positions, row_length, attractions);
            if !optimize_layer_translations(&mut positions, row_length, attractions) {
                break;
            }
        }
        refine_adjacent_people(&mut positions, attractions, &incident_attractions);
        return positions;
    }

    refine_swaps(
        &mut positions,
        row_length,
        attractions,
        &incident_attractions,
    );
    settle_layer_translations(
        &mut positions,
        row_length,
        attractions,
        &incident_attractions,
    );
    let max_compound_moves = match row_length {
        0..=12 => 16,
        _ => 4,
    };
    for _ in 0..max_compound_moves {
        if !refine_three_slot_move(
            &mut positions,
            row_length,
            attractions,
            &incident_attractions,
        ) {
            break;
        }
        refine_swaps(
            &mut positions,
            row_length,
            attractions,
            &incident_attractions,
        );
        settle_layer_translations(
            &mut positions,
            row_length,
            attractions,
            &incident_attractions,
        );
    }
    positions
}

fn refine_targeted_moves(
    person_indices: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) {
    for pass in 0..8 {
        let mut changed = false;
        let mut positions = position_map(person_indices);
        let layer_indices = if pass % 2 == 0 {
            (0..person_indices.len()).collect_vec()
        } else {
            (0..person_indices.len()).rev().collect_vec()
        };
        for layer_index in layer_indices {
            let mut occupants = person_indices[layer_index]
                .iter()
                .enumerate()
                .map(|(index, person)| (person.index, index))
                .collect::<HashMap<_, _>>();
            let person_order = if pass % 2 == 0 {
                (0..person_indices[layer_index].len()).collect_vec()
            } else {
                (0..person_indices[layer_index].len()).rev().collect_vec()
            };

            for person_index in person_order {
                let person = PersonLocation {
                    layer: layer_index,
                    pid: person_indices[layer_index][person_index].pid,
                };
                let current = positions[&person];
                let incident = incident_attractions
                    .get(&person)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let total_weight = incident
                    .iter()
                    .map(|index| attractions[*index].weight)
                    .sum::<usize>();
                if total_weight == 0 {
                    continue;
                }
                let weighted_sum = incident
                    .iter()
                    .map(|index| {
                        let attraction = attractions[*index];
                        let other = if attraction.people[0] == person {
                            attraction.people[1]
                        } else {
                            attraction.people[0]
                        };
                        positions[&other] * attraction.weight
                    })
                    .sum::<usize>();
                let center = weighted_sum / total_weight;
                let mut targets = [
                    center.saturating_sub(1),
                    center,
                    (center + 1).min(row_length - 1),
                    (weighted_sum + total_weight / 2) / total_weight,
                ]
                .into_iter()
                .filter(|target| *target < row_length && *target != current)
                .collect_vec();
                targets.sort_unstable();
                targets.dedup();

                let mut best_improvement = 0;
                let mut best_move = None;
                for target in targets {
                    let other_index = occupants.get(&target).copied();
                    let other = other_index.map(|index| PersonLocation {
                        layer: layer_index,
                        pid: person_indices[layer_index][index].pid,
                    });
                    let mut moved = vec![(person, target)];
                    if let Some(other) = other {
                        moved.push((other, current));
                    }
                    let affected = affected_attractions(&moved, incident_attractions);
                    let before = affected
                        .iter()
                        .filter_map(|index| attraction_score(&attractions[*index], &positions, &[]))
                        .sum::<usize>();
                    let after = affected
                        .iter()
                        .filter_map(|index| {
                            attraction_score(&attractions[*index], &positions, &moved)
                        })
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
                    occupants.remove(&current);
                    occupants.insert(target, person_index);
                    if let (Some(other_index), Some(other)) = (other_index, other) {
                        person_indices[layer_index][other_index].index = current;
                        positions.insert(other, current);
                        occupants.insert(current, other_index);
                    }
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}

fn relax_ordered_coordinates(
    person_indices: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
) {
    let mut neighbours = HashMap::<PersonLocation, Vec<(PersonLocation, usize)>>::new();
    for attraction in attractions {
        let [first, second] = attraction.people;
        neighbours
            .entry(first)
            .or_default()
            .push((second, attraction.weight));
        neighbours
            .entry(second)
            .or_default()
            .push((first, attraction.weight));
    }
    let mut best = person_indices.clone();
    let mut best_score = layout_score(&best, attractions);
    let mut positions = position_map(person_indices);

    for sweep in 0..8 {
        let layer_indices = if sweep % 2 == 0 {
            (0..person_indices.len()).collect_vec()
        } else {
            (0..person_indices.len()).rev().collect_vec()
        };
        for layer_index in layer_indices {
            let mut ordered = person_indices[layer_index]
                .iter()
                .enumerate()
                .map(|(index, person)| (index, person.pid, person.index))
                .collect_vec();
            radix_sort_by_u64(&mut ordered, |(_, _, position)| *position as u64);
            let targets = ordered
                .iter()
                .map(|(_, pid, position)| {
                    let location = PersonLocation {
                        layer: layer_index,
                        pid: *pid,
                    };
                    let incident = neighbours.get(&location).map(Vec::as_slice).unwrap_or(&[]);
                    let total_weight = incident.iter().map(|(_, weight)| *weight).sum::<usize>();
                    if total_weight == 0 {
                        *position as f64
                    } else {
                        let neighbour_sum = incident
                            .iter()
                            .map(|(other, weight)| positions[other] * weight)
                            .sum::<usize>();
                        (*position * total_weight + neighbour_sum) as f64
                            / (2 * total_weight) as f64
                    }
                })
                .collect_vec();
            let projected = project_distinct_positions(&targets, row_length);
            for ((person_index, _, _), position) in ordered.into_iter().zip(projected) {
                person_indices[layer_index][person_index].index = position;
                positions.insert(
                    PersonLocation {
                        layer: layer_index,
                        pid: person_indices[layer_index][person_index].pid,
                    },
                    position,
                );
            }
        }

        let score = layout_score(person_indices, attractions);
        if score < best_score {
            best_score = score;
            best = person_indices.clone();
        }
    }
    *person_indices = best;
}

fn refine_adjacent_people(
    person_indices: &mut Grid<PersonIndex>,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) {
    for pass in 0..8 {
        let mut changed = false;
        let layer_indices = if pass % 2 == 0 {
            (0..person_indices.len()).collect_vec()
        } else {
            (0..person_indices.len()).rev().collect_vec()
        };
        let mut positions = position_map(person_indices);
        for layer_index in layer_indices {
            let mut people = person_indices[layer_index]
                .iter()
                .map(|person| PersonLocation {
                    layer: layer_index,
                    pid: person.pid,
                })
                .collect_vec();
            radix_sort_by_u64(&mut people, |person| positions[person] as u64);
            if pass % 2 == 1 {
                people.reverse();
            }

            for pair in people.windows(2) {
                let [first, second] = [pair[0], pair[1]];
                let first_position = positions[&first];
                let second_position = positions[&second];
                let moved = [(first, second_position), (second, first_position)];
                let affected = affected_attractions(&moved, incident_attractions);
                let before = affected
                    .iter()
                    .filter_map(|index| attraction_score(&attractions[*index], &positions, &[]))
                    .sum::<usize>();
                let after = affected
                    .iter()
                    .filter_map(|index| attraction_score(&attractions[*index], &positions, &moved))
                    .sum::<usize>();
                if after >= before {
                    continue;
                }

                person_indices[layer_index]
                    .iter_mut()
                    .find(|person| person.pid == first.pid)
                    .expect("first adjacent person")
                    .index = second_position;
                person_indices[layer_index]
                    .iter_mut()
                    .find(|person| person.pid == second.pid)
                    .expect("second adjacent person")
                    .index = first_position;
                positions.insert(first, second_position);
                positions.insert(second, first_position);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

fn settle_layer_translations(
    positions: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) {
    for _ in 0..8 {
        if !optimize_layer_translations(positions, row_length, attractions) {
            break;
        }
        refine_swaps(positions, row_length, attractions, incident_attractions);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TranslationScore {
    objective: usize,
    movement: usize,
}

/// Optimize uniform horizontal translations of all generation shapes at once.
///
/// Individual swaps cannot cross a local barrier when two or more adjacent
/// generations already have good internal layouts but are collectively offset
/// from their relatives. With fixed row shapes, adjacent-generation attraction
/// makes this a small exact dynamic program over the legal shift of each row.
fn optimize_layer_translations(
    person_indices: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
) -> bool {
    // Keep the quadratic-in-width cleanup a bounded constant so the fallback
    // remains linear as trees grow. GOT's width is 42.
    if row_length > MAX_TRANSLATION_WIDTH {
        return false;
    }
    let positions = position_map(person_indices);
    let legal_shifts = person_indices
        .iter()
        .map(|layer| {
            let Some(minimum) = layer.iter().map(|person| person.index).min() else {
                return vec![0];
            };
            let maximum = layer
                .iter()
                .map(|person| person.index)
                .max()
                .expect("non-empty layer");
            (-(minimum as isize)..=(row_length - 1 - maximum) as isize).collect_vec()
        })
        .collect_vec();
    let mut cross_attractions = vec![Vec::<usize>::new(); person_indices.len()];
    for (index, attraction) in attractions.iter().enumerate() {
        let [first, second] = attraction.people;
        if first.layer == second.layer {
            continue;
        }
        let later_layer = first.layer.max(second.layer);
        if first.layer.abs_diff(second.layer) != 1 {
            return false;
        }
        cross_attractions[later_layer].push(index);
    }

    let mut predecessors = legal_shifts
        .iter()
        .map(|shifts| vec![usize::MAX; shifts.len()])
        .collect_vec();
    let mut previous_scores = legal_shifts[0]
        .iter()
        .map(|shift| TranslationScore {
            objective: 0,
            movement: shift.unsigned_abs(),
        })
        .collect_vec();

    for layer_index in 1..legal_shifts.len() {
        let mut current_scores = vec![
            TranslationScore {
                objective: usize::MAX,
                movement: usize::MAX,
            };
            legal_shifts[layer_index].len()
        ];
        for (current_index, current_shift) in legal_shifts[layer_index].iter().enumerate() {
            for (previous_index, previous_shift) in legal_shifts[layer_index - 1].iter().enumerate()
            {
                let cross_score = cross_attractions[layer_index]
                    .iter()
                    .map(|attraction_index| {
                        let attraction = &attractions[*attraction_index];
                        let shifted = attraction.people.map(|person| {
                            let shift = if person.layer == layer_index {
                                *current_shift
                            } else {
                                *previous_shift
                            };
                            (positions[&person] as isize + shift) as usize
                        });
                        shifted[0].abs_diff(shifted[1]) * attraction.weight
                    })
                    .sum::<usize>();
                let score = TranslationScore {
                    objective: previous_scores[previous_index].objective + cross_score,
                    movement: previous_scores[previous_index].movement
                        + current_shift.unsigned_abs(),
                };
                if score < current_scores[current_index] {
                    current_scores[current_index] = score;
                    predecessors[layer_index][current_index] = previous_index;
                }
            }
        }
        previous_scores = current_scores;
    }

    let Some(mut shift_index) = previous_scores
        .iter()
        .enumerate()
        .min_by_key(|(_, score)| *score)
        .map(|(index, _)| index)
    else {
        return false;
    };
    let mut selected_shifts = vec![0isize; person_indices.len()];
    for layer_index in (0..person_indices.len()).rev() {
        selected_shifts[layer_index] = legal_shifts[layer_index][shift_index];
        if layer_index > 0 {
            shift_index = predecessors[layer_index][shift_index];
        }
    }

    if selected_shifts.iter().all(|shift| *shift == 0) {
        return false;
    }
    let before = layout_score(person_indices, attractions);
    for (layer, shift) in person_indices.iter_mut().zip(selected_shifts) {
        for person in layer {
            person.index = (person.index as isize + shift) as usize;
        }
    }
    debug_assert!(layout_score(person_indices, attractions) < before);
    true
}

fn refine_swaps(
    positions: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) {
    for pass in 0..MAX_SWAP_PASSES {
        let layer_indices: Vec<usize> = if pass % 2 == 0 {
            (0..positions.len()).collect()
        } else {
            (0..positions.len()).rev().collect()
        };
        let mut changed = false;
        for layer_index in layer_indices {
            changed |= refine_layer(
                positions,
                layer_index,
                row_length,
                attractions,
                incident_attractions,
            );
        }
        if !changed {
            break;
        }
    }
}

fn attractions(relationship_layers: &Grid<Rid>, relationships: &[Relationship]) -> Vec<Attraction> {
    let mut attractions = Vec::new();
    for (child_layer, relationship_ids) in relationship_layers.iter().enumerate() {
        for relationship_id in relationship_ids {
            let relationship = relationships
                .iter()
                .find(|relationship| relationship.id == *relationship_id)
                .expect("Relationship layer must reference an existing relationship");
            let parent_layer = child_layer.checked_sub(1);

            if let ([Some(first), Some(second)], Some(parent_layer)) =
                (relationship.parents, parent_layer)
            {
                attractions.push(Attraction {
                    people: [
                        PersonLocation {
                            layer: parent_layer,
                            pid: first,
                        },
                        PersonLocation {
                            layer: parent_layer,
                            pid: second,
                        },
                    ],
                    weight: MARRIAGE_WEIGHT,
                });
            }
            for [first, second] in relationship.children.iter().copied().array_combinations() {
                attractions.push(Attraction {
                    people: [
                        PersonLocation {
                            layer: child_layer,
                            pid: first,
                        },
                        PersonLocation {
                            layer: child_layer,
                            pid: second,
                        },
                    ],
                    weight: SIBLING_WEIGHT,
                });
            }
            let Some(parent_layer) = parent_layer else {
                continue;
            };
            for parent in relationship.parents.iter().flatten() {
                for child in &relationship.children {
                    attractions.push(Attraction {
                        people: [
                            PersonLocation {
                                layer: parent_layer,
                                pid: *parent,
                            },
                            PersonLocation {
                                layer: child_layer,
                                pid: *child,
                            },
                        ],
                        weight: PARENT_CHILD_WEIGHT,
                    });
                }
            }
        }
    }
    attractions
}

fn incident_attractions(attractions: &[Attraction]) -> HashMap<PersonLocation, Vec<usize>> {
    let mut incident = HashMap::<PersonLocation, Vec<usize>>::new();
    for (index, attraction) in attractions.iter().enumerate() {
        for person in attraction.people {
            incident.entry(person).or_default().push(index);
        }
    }
    incident
}

fn position_map(person_indices: &Grid<PersonIndex>) -> HashMap<PersonLocation, usize> {
    person_indices
        .iter()
        .enumerate()
        .flat_map(|(layer, people)| {
            people.iter().map(move |person| {
                (
                    PersonLocation {
                        layer,
                        pid: person.pid,
                    },
                    person.index,
                )
            })
        })
        .collect()
}

fn refine_layer(
    person_indices: &mut Grid<PersonIndex>,
    layer_index: usize,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) -> bool {
    let mut changed = false;
    let mut positions = position_map(person_indices);

    for person_index in 0..person_indices[layer_index].len() {
        let person = PersonLocation {
            layer: layer_index,
            pid: person_indices[layer_index][person_index].pid,
        };
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
            let other = other_index.map(|index| PersonLocation {
                layer: layer_index,
                pid: person_indices[layer_index][index].pid,
            });
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

/// Try the best improving rotation of three slots.
///
/// A swap-only search can get stuck when two people need to move together,
/// especially when a layer contains empty slots. Rotating three slots covers
/// those coordinated moves while preserving the generation and uniqueness
/// constraints.
fn refine_three_slot_move(
    person_indices: &mut Grid<PersonIndex>,
    row_length: usize,
    attractions: &[Attraction],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) -> bool {
    let positions = position_map(person_indices);
    let mut best_improvement = 0;
    let mut best_move = None;

    for (layer_index, layer) in person_indices.iter().enumerate() {
        let mut people_by_slot = vec![None; row_length];
        for person in layer {
            people_by_slot[person.index] = Some(PersonLocation {
                layer: layer_index,
                pid: person.pid,
            });
        }

        for first in 0..row_length {
            for second in (first + 1)..row_length {
                for third in (second + 1)..row_length {
                    let slots = [first, second, third];
                    let people = slots.map(|slot| people_by_slot[slot]);
                    if people.iter().flatten().count() < 2 {
                        continue;
                    }

                    for target_slots in [[third, first, second], [second, third, first]] {
                        let moved = people
                            .iter()
                            .zip(target_slots)
                            .filter_map(|(person, target)| person.map(|person| (person, target)))
                            .collect_vec();
                        let affected = affected_attractions(&moved, incident_attractions);
                        let before = affected
                            .iter()
                            .filter_map(|index| {
                                attraction_score(&attractions[*index], &positions, &[])
                            })
                            .sum::<usize>();
                        let after = affected
                            .iter()
                            .filter_map(|index| {
                                attraction_score(&attractions[*index], &positions, &moved)
                            })
                            .sum::<usize>();
                        let improvement = before.saturating_sub(after);
                        if after < before && improvement > best_improvement {
                            best_improvement = improvement;
                            best_move = Some((layer_index, moved));
                        }
                    }
                }
            }
        }
    }

    let Some((layer_index, moved)) = best_move else {
        return false;
    };
    for (person, target) in moved {
        person_indices[layer_index]
            .iter_mut()
            .find(|candidate| candidate.pid == person.pid)
            .expect("Moved person must exist in its layer")
            .index = target;
    }
    true
}

fn affected_attractions(
    moved: &[(PersonLocation, usize)],
    incident_attractions: &HashMap<PersonLocation, Vec<usize>>,
) -> Vec<usize> {
    let mut affected = moved
        .iter()
        .flat_map(|(person, _)| {
            incident_attractions
                .get(person)
                .into_iter()
                .flatten()
                .copied()
        })
        .collect_vec();
    affected.sort_unstable();
    affected.dedup();
    affected
}

fn attraction_score(
    attraction: &Attraction,
    positions: &HashMap<PersonLocation, usize>,
    moved: &[(PersonLocation, usize)],
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

fn layout_score(person_indices: &Grid<PersonIndex>, attractions: &[Attraction]) -> usize {
    let positions = position_map(person_indices);
    attractions
        .iter()
        .filter_map(|attraction| attraction_score(attraction, &positions, &[]))
        .sum()
}

/// Resolve equal-score child/spouse orientations deterministically. When one
/// spouse has parents in the previous generation and the other does not, the
/// child occupies the adjacent slot nearest the parents' arithmetic center.
fn resolve_child_spouse_order(
    person_indices: &mut Grid<PersonIndex>,
    relationships: &[Relationship],
    attractions: &[Attraction],
) {
    let mut score = layout_score(person_indices, attractions);
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
                let swapped_score = layout_score(person_indices, attractions);
                if swapped_score > score {
                    person_indices[layer_index][child_index].index = child_position;
                    person_indices[layer_index][partner_index].index = partner_position;
                } else {
                    score = swapped_score;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baumstamm_lib::{FamilyTree, PersonId, RelationshipId, graph::Graph};

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

    fn infer_relationship_layers(
        person_layers: &Grid<Pid>,
        relationships: &[Relationship],
    ) -> Grid<Rid> {
        let mut layers = vec![Vec::new(); person_layers.len() + 1];
        for relationship in relationships {
            let layer_index = (0..=person_layers.len())
                .find(|layer_index| {
                    let parents_present = relationship.parents.iter().flatten().all(|parent| {
                        layer_index
                            .checked_sub(1)
                            .and_then(|parent_layer| person_layers.get(parent_layer))
                            .is_some_and(|layer| layer.contains(parent))
                    });
                    let children_present = relationship.children.iter().all(|child| {
                        person_layers
                            .get(*layer_index)
                            .is_some_and(|layer| layer.contains(child))
                    });
                    parents_present && children_present
                })
                .expect("Relationship must connect adjacent person layers");
            layers[layer_index].push(relationship.id);
        }
        while layers.last().is_some_and(Vec::is_empty) {
            layers.pop();
        }
        layers
    }

    fn get_person_indices(
        person_layers: &Grid<Pid>,
        relationships: &[Relationship],
        row_length: usize,
    ) -> Grid<PersonIndex> {
        let relationship_layers = infer_relationship_layers(person_layers, relationships);
        super::get_person_indices(PersonIndexInput {
            person_layers,
            relationship_layers: &relationship_layers,
            relationships,
            row_length,
            layout_algorithm: super::super::LayoutAlgorithm::ForceDirected,
        })
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
        let indices = get_person_indices(&layers, &relationships, 3);

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
        let indices = get_person_indices(&layers, &relationships, 4);
        let positions = position_map(&indices);
        let location = |person| PersonLocation {
            layer: 0,
            pid: pid(person),
        };

        assert_eq!(positions[&location(1)].abs_diff(positions[&location(3)]), 1);
        assert_eq!(positions[&location(1)].abs_diff(positions[&location(4)]), 1);
        assert!(positions[&location(1)].abs_diff(positions[&location(2)]) >= 2);
    }

    #[test]
    fn child_is_ordered_inside_spouse_for_two_slot_tie() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[3]),
            relationship(2, [Some(3), Some(4)], &[]),
        ];
        let indices = get_person_indices(&layers, &relationships, 2);

        assert_eq!(slots(&indices[1]), vec![(4, 0), (3, 1)]);
    }

    #[test]
    fn child_is_moved_next_to_offset_parents_with_spouse_outside() {
        let layers = vec![vec![pid(9), pid(1), pid(2)], vec![pid(3), pid(4), pid(8)]];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[3]),
            relationship(2, [Some(3), Some(4)], &[]),
        ];
        let indices = get_person_indices(&layers, &relationships, 3);

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
        let expected = get_person_indices(&layers, &relationships, 4);

        for _ in 0..10 {
            let actual = get_person_indices(&layers, &relationships, 4);
            assert_eq!(
                actual.iter().map(|layer| slots(layer)).collect_vec(),
                expected.iter().map(|layer| slots(layer)).collect_vec()
            );
        }
    }

    #[test]
    fn small_layouts_are_solved_to_global_optimality() {
        let layers = vec![vec![pid(0), pid(1), pid(2), pid(3), pid(4), pid(5)]];
        let relationships = vec![
            relationship(1, [Some(0), Some(3)], &[]),
            relationship(2, [Some(1), Some(3)], &[]),
            relationship(3, [Some(1), Some(4)], &[]),
        ];
        let relationship_layers = infer_relationship_layers(&layers, &relationships);
        let attractions = attractions(&relationship_layers, &relationships);
        let indices = get_person_indices(&layers, &relationships, 6);

        // The three marriage edges can all be adjacent. Their weight is 12.
        assert_eq!(layout_score(&indices, &attractions), 3 * MARRIAGE_WEIGHT);
        assert_eq!(permutation_count(6, 6), Some(720));
    }

    #[test]
    fn three_slot_move_escapes_a_swap_local_minimum() {
        let layers = vec![vec![pid(0), pid(1), pid(2), pid(3), pid(4), pid(5)]];
        let relationships = vec![
            relationship(1, [Some(0), Some(3)], &[]),
            relationship(2, [Some(1), Some(3)], &[]),
            relationship(3, [Some(1), Some(4)], &[]),
        ];
        let relationship_layers = infer_relationship_layers(&layers, &relationships);
        let attractions = attractions(&relationship_layers, &relationships);
        let incident = incident_attractions(&attractions);
        let mut indices = centered::get_person_indices(PersonIndexInput {
            person_layers: &layers,
            relationship_layers: &relationship_layers,
            relationships: &relationships,
            row_length: 6,
            layout_algorithm: super::super::LayoutAlgorithm::Centered,
        });

        refine_swaps(&mut indices, 6, &attractions, &incident);
        assert_eq!(layout_score(&indices, &attractions), 4 * MARRIAGE_WEIGHT);

        assert!(refine_three_slot_move(
            &mut indices,
            6,
            &attractions,
            &incident
        ));
        refine_swaps(&mut indices, 6, &attractions, &incident);
        assert_eq!(layout_score(&indices, &attractions), 3 * MARRIAGE_WEIGHT);
    }

    #[test]
    fn related_generation_shapes_are_translated_together() {
        let mut indices = vec![
            vec![
                PersonIndex {
                    index: 2,
                    pid: pid(1),
                },
                PersonIndex {
                    index: 3,
                    pid: pid(2),
                },
            ],
            vec![
                PersonIndex {
                    index: 2,
                    pid: pid(3),
                },
                PersonIndex {
                    index: 3,
                    pid: pid(4),
                },
            ],
            vec![
                PersonIndex {
                    index: 0,
                    pid: pid(5),
                },
                PersonIndex {
                    index: 1,
                    pid: pid(6),
                },
                PersonIndex {
                    index: 2,
                    pid: pid(7),
                },
                PersonIndex {
                    index: 3,
                    pid: pid(8),
                },
            ],
        ];
        let location = |layer, person| PersonLocation {
            layer,
            pid: pid(person),
        };
        let attractions = vec![
            Attraction {
                people: [location(0, 1), location(0, 2)],
                weight: MARRIAGE_WEIGHT,
            },
            Attraction {
                people: [location(1, 3), location(1, 4)],
                weight: MARRIAGE_WEIGHT,
            },
            Attraction {
                people: [location(0, 1), location(1, 3)],
                weight: PARENT_CHILD_WEIGHT,
            },
            Attraction {
                people: [location(0, 2), location(1, 4)],
                weight: PARENT_CHILD_WEIGHT,
            },
            Attraction {
                people: [location(1, 3), location(2, 5)],
                weight: PARENT_CHILD_WEIGHT,
            },
            Attraction {
                people: [location(1, 4), location(2, 6)],
                weight: PARENT_CHILD_WEIGHT,
            },
        ];
        let before = layout_score(&indices, &attractions);

        assert!(optimize_layer_translations(&mut indices, 4, &attractions));

        assert!(layout_score(&indices, &attractions) < before);
        assert_eq!(slots(&indices[0]), vec![(1, 0), (2, 1)]);
        assert_eq!(slots(&indices[1]), vec![(3, 0), (4, 1)]);
        assert_eq!(slots(&indices[2]), vec![(5, 0), (6, 1), (7, 2), (8, 3)]);
    }

    #[test]
    fn repeated_person_occurrences_are_scored_separately() {
        let person_layers = vec![vec![pid(1), pid(2)], vec![pid(3), pid(1)], vec![pid(4)]];
        let relationships = vec![
            relationship(1, [Some(1), Some(2)], &[3]),
            relationship(2, [Some(3), Some(1)], &[4]),
        ];
        let relationship_layers = vec![
            vec![],
            vec![RelationshipId::from(1)],
            vec![RelationshipId::from(2)],
        ];
        let attractions = attractions(&relationship_layers, &relationships);
        let indices = super::get_person_indices(PersonIndexInput {
            person_layers: &person_layers,
            relationship_layers: &relationship_layers,
            relationships: &relationships,
            row_length: 2,
            layout_algorithm: super::super::LayoutAlgorithm::ForceDirected,
        });
        let positions = position_map(&indices);

        assert_eq!(positions.len(), 5);
        assert!(positions.contains_key(&PersonLocation {
            layer: 0,
            pid: pid(1),
        }));
        assert!(positions.contains_key(&PersonLocation {
            layer: 1,
            pid: pid(1),
        }));
        assert!(attractions.iter().any(|attraction| {
            attraction.people.contains(&PersonLocation {
                layer: 0,
                pid: pid(1),
            })
        }));
        assert!(attractions.iter().any(|attraction| {
            attraction.people.contains(&PersonLocation {
                layer: 1,
                pid: pid(1),
            })
        }));
    }

    #[test]
    fn lotr_reaches_known_global_lower_bound() {
        let tree = FamilyTree::try_from(include_str!("../../../examples/lotr/lotr.json"))
            .expect("valid example tree");
        let relationships = tree.get_relationships();
        let graph = Graph::new(relationships).cut();
        let relationship_layers = graph.layers();
        let person_layers = graph.person_layers(relationships);
        let row_length = person_layers.iter().map(Vec::len).max().unwrap();
        let attractions = attractions(&relationship_layers, relationships);
        let complete_layout_count = person_layers
            .iter()
            .map(|layer| permutation_count(row_length, layer.len()).unwrap() as u128)
            .product::<u128>();
        let exact = exact_layered_layout(&person_layers, row_length, &attractions)
            .expect("six-column tree should use exact layered optimization");
        let indices = super::get_person_indices(PersonIndexInput {
            person_layers: &person_layers,
            relationship_layers: &relationship_layers,
            relationships,
            row_length,
            layout_algorithm: super::super::LayoutAlgorithm::ForceDirected,
        });

        // Every relationship independently attains its mathematical lower
        // bound in the known six-column layout.
        assert_eq!(complete_layout_count, 145_118_822_400_000_000);
        assert_eq!(layout_score(&exact, &attractions), 209);
        assert_eq!(layout_score(&indices, &attractions), 209);
    }

    #[test]
    fn got_large_layout_is_stable_under_collective_generation_shifts() {
        let tree = FamilyTree::try_from(include_str!("../../../examples/got/got.json"))
            .expect("valid example tree");
        let relationships = tree.get_relationships();
        let graph = Graph::new(relationships).cut();
        let relationship_layers = graph.layers();
        let person_layers = graph.person_layers(relationships);
        let row_length = person_layers.iter().map(Vec::len).max().unwrap();
        let attractions = attractions(&relationship_layers, relationships);
        assert!(exact_layered_layout(&person_layers, row_length, &attractions).is_none());

        #[cfg(not(debug_assertions))]
        let started = std::time::Instant::now();
        let mut indices = super::get_person_indices(PersonIndexInput {
            person_layers: &person_layers,
            relationship_layers: &relationship_layers,
            relationships,
            row_length,
            layout_algorithm: super::super::LayoutAlgorithm::ForceDirected,
        });
        #[cfg(not(debug_assertions))]
        assert!(
            started.elapsed() < std::time::Duration::from_millis(500),
            "GOT layout exceeded the 0.5 second release budget"
        );

        assert!(layout_score(&indices, &attractions) <= 9_940);
        assert!(!optimize_layer_translations(
            &mut indices,
            row_length,
            &attractions
        ));
    }
}
