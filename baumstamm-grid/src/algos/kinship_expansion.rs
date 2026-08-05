use super::{PersonIndexInput, PersonIndexOutput, common::project_to_slots};
use crate::indices::PersonIndex;
use baumstamm_lib::{PersonId, Relationship, RelationshipId};
use std::collections::{HashMap, HashSet, VecDeque};

type Pid = PersonId;
/// The expansion layout has only one width allowance: one spare column per
/// eight people in the widest generation. Family blocks and isolated people
/// may move within that allowance, but the layout never grows with tree size.
const EXTRA_WIDTH_DIVISOR: usize = 8;
const UNREACHABLE_RANK: usize = usize::MAX;
const GUIDE_PASSES: usize = 20;
const GUIDE_REPEAT_WEIGHT: usize = 10;
const GUIDE_PARENT_WEIGHT: usize = 20;
const GUIDE_CHILD_WEIGHT: usize = 8;
const GUIDE_SPOUSE_WEIGHT: usize = 12;

/// Preindexed semantic and rendered-family information. The semantic graph is
/// an incidence graph (person -> relationship -> person), so siblings,
/// partners, and parent/child pairs are one kinship frontier apart without
/// materializing a quadratic clique for large sibling groups.
struct Model<'a> {
    relationships: &'a [Relationship],
    relationship_members: Vec<Vec<Pid>>,
    memberships: HashMap<Pid, Vec<usize>>,
    ranks: HashMap<Pid, usize>,
    families_by_layer: Vec<Vec<usize>>,
    partners_by_layer: Vec<HashMap<Pid, Vec<Pid>>>,
    guide: Vec<HashMap<Pid, usize>>,
}

#[derive(Clone, Copy)]
struct PlacedOccurrence {
    layer: usize,
    column: usize,
}

struct FamilyUnit {
    members: Vec<Pid>,
    target_twice: usize,
    rank: usize,
    canonical_key: Vec<Pid>,
    guide_target_twice: usize,
}

#[derive(Clone)]
struct GuideGroup {
    member_layer: usize,
    members: Vec<Pid>,
    weight: usize,
}

struct GuideIndex {
    occurrence_layers: HashMap<Pid, Vec<usize>>,
    groups_by_layer: Vec<HashMap<Pid, Vec<GuideGroup>>>,
}

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

    let row_length = widest_layer
        .checked_add(widest_layer.div_ceil(EXTRA_WIDTH_DIVISOR))
        .expect("kinship expansion width exceeds addressable memory");
    let oldest_layer = input
        .person_layers
        .iter()
        .position(|layer| !layer.is_empty())
        .expect("a widest non-empty layer must exist");
    // PersonId is stable tree data, unlike relationship/source-vector order.
    // Its minimum is therefore the explicit canonical root rule.
    let root = *input.person_layers[oldest_layer]
        .iter()
        .min()
        .expect("oldest layer is non-empty");
    let model = Model::new(&input, root, row_length, oldest_layer);
    let center = (row_length - 1) / 2;
    let mut placed_by_pid = HashMap::<Pid, Vec<PlacedOccurrence>>::new();
    let mut person_indices = Vec::with_capacity(input.person_layers.len());

    for (layer_index, layer) in input.person_layers.iter().enumerate() {
        let pin = (layer_index == oldest_layer).then_some((root, center));
        let output_layer = layout_layer(
            &model,
            layer,
            layer_index,
            row_length,
            pin,
            &mut placed_by_pid,
        );
        person_indices.push(output_layer);
    }

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

impl<'a> Model<'a> {
    fn new(
        input: &PersonIndexInput<'a>,
        root: Pid,
        row_length: usize,
        oldest_layer: usize,
    ) -> Self {
        let relationship_by_id = input
            .relationships
            .iter()
            .enumerate()
            .map(|(index, relationship)| (relationship.id, index))
            .collect::<HashMap<_, _>>();
        let relationship_members = input
            .relationships
            .iter()
            .map(|relationship| {
                let mut members = relationship
                    .parents
                    .iter()
                    .flatten()
                    .copied()
                    .chain(relationship.children.iter().copied())
                    .collect::<Vec<_>>();
                members.sort_unstable();
                members.dedup();
                members
            })
            .collect::<Vec<_>>();
        let mut memberships = HashMap::<Pid, Vec<usize>>::new();
        for (relationship_index, members) in relationship_members.iter().enumerate() {
            for member in members {
                memberships
                    .entry(*member)
                    .or_default()
                    .push(relationship_index);
            }
        }
        for relationship_indices in memberships.values_mut() {
            relationship_indices.sort_by(|first, second| {
                relationship_members[*first]
                    .cmp(&relationship_members[*second])
                    .then_with(|| {
                        input.relationships[*first]
                            .id
                            .0
                            .cmp(&input.relationships[*second].id.0)
                    })
            });
        }

        let ranks = kinship_ranks(root, &memberships, &relationship_members);
        let families_by_layer = input
            .relationship_layers
            .iter()
            .map(|relationship_ids| {
                let mut indices = relationship_ids
                    .iter()
                    .map(|id| {
                        *relationship_by_id
                            .get(id)
                            .expect("relationship layer references an unknown relationship")
                    })
                    .collect::<Vec<_>>();
                indices.sort_by(|first, second| {
                    canonical_relationship_cmp(
                        &input.relationships[*first],
                        &input.relationships[*second],
                    )
                });
                indices
            })
            .collect();

        let layer_sets = input
            .person_layers
            .iter()
            .map(|layer| layer.iter().copied().collect::<HashSet<_>>())
            .collect::<Vec<_>>();
        let mut partners_by_layer = vec![HashMap::<Pid, Vec<Pid>>::new(); layer_sets.len()];
        for relationship in input.relationships {
            let [Some(first), Some(second)] = relationship.parents else {
                continue;
            };
            for (layer_index, people) in layer_sets.iter().enumerate() {
                if people.contains(&first) && people.contains(&second) {
                    partners_by_layer[layer_index]
                        .entry(first)
                        .or_default()
                        .push(second);
                    partners_by_layer[layer_index]
                        .entry(second)
                        .or_default()
                        .push(first);
                }
            }
        }
        for partners in &mut partners_by_layer {
            for values in partners.values_mut() {
                values.sort_unstable();
                values.dedup();
            }
        }
        let guide_index = GuideIndex::new(input, &relationship_by_id);
        let guide =
            build_ordering_guide(input, &guide_index, &ranks, root, row_length, oldest_layer);

        Self {
            relationships: input.relationships,
            relationship_members,
            memberships,
            ranks,
            families_by_layer,
            partners_by_layer,
            guide,
        }
    }

    fn rank(&self, pid: Pid) -> usize {
        self.ranks.get(&pid).copied().unwrap_or(UNREACHABLE_RANK)
    }

    fn is_partnered_on_layer(&self, layer: usize, pid: Pid) -> bool {
        self.partners_by_layer
            .get(layer)
            .is_some_and(|partners| partners.contains_key(&pid))
    }

    fn guide_column(&self, layer: usize, pid: Pid) -> Option<usize> {
        self.guide
            .get(layer)
            .and_then(|columns| columns.get(&pid))
            .copied()
    }
}

impl GuideIndex {
    fn new(
        input: &PersonIndexInput<'_>,
        relationship_by_id: &HashMap<RelationshipId, usize>,
    ) -> Self {
        let mut occurrence_layers = HashMap::<Pid, Vec<usize>>::new();
        for (layer, people) in input.person_layers.iter().enumerate() {
            for pid in people {
                occurrence_layers.entry(*pid).or_default().push(layer);
            }
        }
        for layers in occurrence_layers.values_mut() {
            layers.sort_unstable();
            layers.dedup();
        }

        let mut groups_by_layer =
            vec![HashMap::<Pid, Vec<GuideGroup>>::new(); input.person_layers.len()];
        for (child_layer, relationship_ids) in input.relationship_layers.iter().enumerate() {
            let Some(parent_layer) = child_layer.checked_sub(1) else {
                continue;
            };
            for id in relationship_ids {
                let relationship = &input.relationships[*relationship_by_id
                    .get(id)
                    .expect("relationship layer references an unknown relationship")];
                let mut parents = relationship
                    .parents
                    .iter()
                    .flatten()
                    .copied()
                    .collect::<Vec<_>>();
                parents.sort_unstable();
                let mut children = relationship.children.clone();
                children.sort_unstable();
                for child in &children {
                    if !parents.is_empty() {
                        groups_by_layer[child_layer]
                            .entry(*child)
                            .or_default()
                            .push(GuideGroup {
                                member_layer: parent_layer,
                                members: parents.clone(),
                                weight: GUIDE_PARENT_WEIGHT,
                            });
                    }
                }
                for parent in &parents {
                    if !children.is_empty() {
                        groups_by_layer[parent_layer]
                            .entry(*parent)
                            .or_default()
                            .push(GuideGroup {
                                member_layer: child_layer,
                                members: children.clone(),
                                weight: GUIDE_CHILD_WEIGHT,
                            });
                    }
                    for partner in parents.iter().filter(|partner| *partner != parent) {
                        groups_by_layer[parent_layer]
                            .entry(*parent)
                            .or_default()
                            .push(GuideGroup {
                                member_layer: parent_layer,
                                members: vec![*partner],
                                weight: GUIDE_SPOUSE_WEIGHT,
                            });
                    }
                }
            }
        }
        for groups in &mut groups_by_layer {
            for person_groups in groups.values_mut() {
                person_groups.sort_by(|first, second| {
                    (first.member_layer, first.weight, &first.members).cmp(&(
                        second.member_layer,
                        second.weight,
                        &second.members,
                    ))
                });
            }
        }
        Self {
            occurrence_layers,
            groups_by_layer,
        }
    }
}

fn build_ordering_guide(
    input: &PersonIndexInput<'_>,
    index: &GuideIndex,
    ranks: &HashMap<Pid, usize>,
    root: Pid,
    row_length: usize,
    oldest_layer: usize,
) -> Vec<HashMap<Pid, usize>> {
    let center = (row_length - 1) / 2;
    let mut guide = input
        .person_layers
        .iter()
        .map(|layer| {
            let mut people = layer.clone();
            people.sort_unstable();
            let start = (row_length - people.len()) / 2;
            people
                .into_iter()
                .enumerate()
                .map(|(offset, pid)| (pid, start + offset))
                .collect::<HashMap<_, _>>()
        })
        .collect::<Vec<_>>();
    guide[oldest_layer].insert(root, center);

    for pass in 0..GUIDE_PASSES {
        let layers = if pass % 2 == 0 {
            (0..guide.len()).collect::<Vec<_>>()
        } else {
            (0..guide.len()).rev().collect::<Vec<_>>()
        };
        for layer in layers {
            if layer == oldest_layer || input.person_layers[layer].is_empty() {
                continue;
            }
            let mut occurrences = input.person_layers[layer].clone();
            occurrences.sort_unstable();
            let mut desired = occurrences
                .into_iter()
                .map(|pid| {
                    let target = guide_target(index, &guide, layer, pid, center);
                    (
                        pid,
                        target,
                        ranks.get(&pid).copied().unwrap_or(UNREACHABLE_RANK),
                    )
                })
                .collect::<Vec<_>>();
            desired.sort_by(|first, second| {
                first
                    .1
                    .total_cmp(&second.1)
                    .then_with(|| first.2.cmp(&second.2))
                    .then_with(|| first.0.cmp(&second.0))
            });
            let targets = desired.iter().map(|item| item.1).collect::<Vec<_>>();
            let slots = project_to_slots(&targets, row_length);
            guide[layer] = desired
                .into_iter()
                .zip(slots)
                .map(|((pid, _, _), column)| (pid, column))
                .collect();
        }
    }
    guide
}

fn guide_target(
    index: &GuideIndex,
    guide: &[HashMap<Pid, usize>],
    layer: usize,
    pid: Pid,
    center: usize,
) -> f64 {
    let mut weighted_sum = 0.0;
    let mut total_weight = 0;
    let mut add = |column: f64, weight: usize| {
        weighted_sum += column * weight as f64;
        total_weight += weight;
    };

    if let Some((_, column)) = index
        .occurrence_layers
        .get(&pid)
        .into_iter()
        .flatten()
        .filter(|other_layer| **other_layer != layer)
        .filter_map(|other_layer| {
            guide[*other_layer]
                .get(&pid)
                .map(|column| (layer.abs_diff(*other_layer), *column))
        })
        .min_by_key(|(distance, column)| (*distance, column.abs_diff(center), *column))
    {
        add(column as f64, GUIDE_REPEAT_WEIGHT);
    }

    for group in index
        .groups_by_layer
        .get(layer)
        .and_then(|groups| groups.get(&pid))
        .into_iter()
        .flatten()
    {
        let member_columns = group
            .members
            .iter()
            .filter_map(|member| guide[group.member_layer].get(member).copied())
            .collect::<Vec<_>>();
        if !member_columns.is_empty() {
            let sum = member_columns
                .iter()
                .fold(0_u128, |sum, column| sum + *column as u128);
            add(sum as f64 / member_columns.len() as f64, group.weight);
        }
    }

    if total_weight == 0 {
        center as f64
    } else {
        weighted_sum / total_weight as f64
    }
}

fn kinship_ranks(
    root: Pid,
    memberships: &HashMap<Pid, Vec<usize>>,
    relationship_members: &[Vec<Pid>],
) -> HashMap<Pid, usize> {
    let mut ranks = HashMap::from([(root, 0_usize)]);
    let mut expanded_relationships = vec![false; relationship_members.len()];
    let mut queue = VecDeque::from([root]);
    while let Some(person) = queue.pop_front() {
        let next_rank = ranks[&person].saturating_add(1);
        for relationship_index in memberships.get(&person).into_iter().flatten() {
            if expanded_relationships[*relationship_index] {
                continue;
            }
            expanded_relationships[*relationship_index] = true;
            for member in &relationship_members[*relationship_index] {
                if let std::collections::hash_map::Entry::Vacant(entry) = ranks.entry(*member) {
                    entry.insert(next_rank);
                    queue.push_back(*member);
                }
            }
        }
    }
    ranks
}

fn layout_layer(
    model: &Model<'_>,
    source_layer: &[Pid],
    layer_index: usize,
    row_length: usize,
    pin: Option<(Pid, usize)>,
    placed_by_pid: &mut HashMap<Pid, Vec<PlacedOccurrence>>,
) -> Vec<PersonIndex> {
    if source_layer.is_empty() {
        return Vec::new();
    }

    let center = (row_length - 1) / 2;
    let mut remaining =
        source_layer
            .iter()
            .copied()
            .fold(HashMap::<Pid, usize>::new(), |mut counts, pid| {
                *counts.entry(pid).or_default() += 1;
                counts
            });
    let mut columns = vec![None; row_length];

    if let Some((pid, column)) = pin {
        take_occurrence(&mut remaining, pid);
        columns[column] = Some(pid);
        record_placement(placed_by_pid, pid, layer_index, column);
        reserve_pinned_source_family(
            model,
            layer_index,
            pid,
            column,
            &mut remaining,
            &mut columns,
            placed_by_pid,
        );
    }

    // The oldest row expands directly from the pinned root. Later rows use
    // rendered sibling blocks; reserving a source-family block around an
    // already pinned member could make a merely split free interval appear
    // too small even though the row has enough total capacity.
    let mut family_units = if pin.is_some() {
        Vec::new()
    } else {
        collect_family_units(model, layer_index, &mut remaining, placed_by_pid, center)
    };
    reserve_family_blocks(&mut family_units, &mut columns);
    for unit in family_units {
        let start = columns
            .windows(unit.members.len())
            .position(|window| {
                window
                    .iter()
                    .copied()
                    .eq(unit.members.iter().copied().map(Some))
            })
            .expect("reserved family block must remain intact");
        for (offset, pid) in unit.members.into_iter().enumerate() {
            record_placement(placed_by_pid, pid, layer_index, start + offset);
        }
    }

    let mut singles = remaining
        .into_iter()
        .flat_map(|(pid, count)| std::iter::repeat_n(pid, count))
        .collect::<Vec<_>>();
    singles.sort_by_key(|pid| (model.rank(*pid), *pid));
    for pid in singles {
        let semantic_target_twice = nearest_anchor_twice(
            model,
            pid,
            layer_index,
            placed_by_pid,
            center.saturating_mul(2),
        );
        let target_twice = model
            .guide_column(layer_index, pid)
            .map_or(semantic_target_twice, |column| column.saturating_mul(2));
        let column = (0..row_length)
            .filter(|column| columns[*column].is_none())
            .min_by_key(|column| {
                (
                    column.saturating_mul(2).abs_diff(target_twice),
                    column.abs_diff(center),
                    *column,
                )
            })
            .expect("row length is at least the number of occurrences");
        columns[column] = Some(pid);
        record_placement(placed_by_pid, pid, layer_index, column);
    }

    let output = columns
        .into_iter()
        .enumerate()
        .filter_map(|(index, pid)| pid.map(|pid| PersonIndex { pid, index }))
        .collect::<Vec<_>>();
    debug_assert_eq!(output.len(), source_layer.len());
    output
}

/// A source relationship can put a sibling family in the oldest row. When
/// that whole block fits around the pinned root, preserve it just like every
/// later sibling block. If it cannot fit (for example a partnered root must be
/// exterior in a very large source family), root centering is the higher hard
/// priority and normal BFS frontier placement handles the remaining people.
fn reserve_pinned_source_family(
    model: &Model<'_>,
    layer: usize,
    root: Pid,
    root_column: usize,
    remaining: &mut HashMap<Pid, usize>,
    columns: &mut [Option<Pid>],
    placed_by_pid: &mut HashMap<Pid, Vec<PlacedOccurrence>>,
) {
    let Some(relationship_index) = model
        .families_by_layer
        .get(layer)
        .into_iter()
        .flatten()
        .copied()
        .find(|relationship_index| {
            model.relationships[*relationship_index]
                .children
                .contains(&root)
        })
    else {
        return;
    };
    let mut members = model.relationships[relationship_index].children.clone();
    members.sort_unstable();
    members.dedup();
    let members = ordered_siblings(model, layer, members);
    let Some(root_offset) = members.iter().position(|member| *member == root) else {
        return;
    };
    let Some(start) = root_column.checked_sub(root_offset) else {
        return;
    };
    if start
        .checked_add(members.len())
        .is_none_or(|end| end > columns.len())
        || members.iter().any(|member| {
            *member != root && remaining.get(member).copied().unwrap_or_default() == 0
        })
    {
        return;
    }
    for (offset, member) in members.into_iter().enumerate() {
        if member == root {
            continue;
        }
        let column = start + offset;
        debug_assert!(columns[column].is_none());
        take_occurrence(remaining, member);
        columns[column] = Some(member);
        record_placement(placed_by_pid, member, layer, column);
    }
}

fn collect_family_units(
    model: &Model<'_>,
    layer: usize,
    remaining: &mut HashMap<Pid, usize>,
    placed_by_pid: &HashMap<Pid, Vec<PlacedOccurrence>>,
    center: usize,
) -> Vec<FamilyUnit> {
    let mut candidates = model
        .families_by_layer
        .get(layer)
        .into_iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by(|first, second| {
        let first_relationship = &model.relationships[*first];
        let second_relationship = &model.relationships[*second];
        let first_rank = first_relationship
            .children
            .iter()
            .map(|child| model.rank(*child))
            .min()
            .unwrap_or(UNREACHABLE_RANK);
        let second_rank = second_relationship
            .children
            .iter()
            .map(|child| model.rank(*child))
            .min()
            .unwrap_or(UNREACHABLE_RANK);
        first_rank
            .cmp(&second_rank)
            .then_with(|| canonical_relationship_cmp(first_relationship, second_relationship))
    });

    let mut units = Vec::new();
    for relationship_index in candidates {
        let relationship = &model.relationships[relationship_index];
        let mut children = relationship
            .children
            .iter()
            .copied()
            .filter(|child| remaining.get(child).copied().unwrap_or_default() > 0)
            .collect::<Vec<_>>();
        children.sort_unstable();
        children.dedup();
        if children.is_empty() {
            continue;
        }
        let members = ordered_siblings(model, layer, children);
        for member in &members {
            take_occurrence(remaining, *member);
        }
        let target_twice = parent_center_twice(relationship, layer, placed_by_pid)
            .unwrap_or_else(|| center.saturating_mul(2));
        let rank = members
            .iter()
            .map(|member| model.rank(*member))
            .min()
            .unwrap_or(UNREACHABLE_RANK);
        let mut canonical_key = members.clone();
        canonical_key.sort_unstable();
        let guide_sum = members
            .iter()
            .filter_map(|member| model.guide_column(layer, *member))
            .fold(0_u128, |sum, column| sum + column as u128);
        let guide_target_twice =
            usize::try_from(guide_sum.saturating_mul(2) / members.len() as u128)
                .unwrap_or(target_twice);
        units.push(FamilyUnit {
            members,
            target_twice,
            rank,
            canonical_key,
            guide_target_twice,
        });
    }
    units
}

fn ordered_siblings(model: &Model<'_>, layer: usize, mut children: Vec<Pid>) -> Vec<Pid> {
    children.sort_by_key(|child| (model.rank(*child), *child));
    if children.len() <= 2 {
        return children;
    }

    let (partnered, unpartnered): (Vec<_>, Vec<_>) = children
        .into_iter()
        .partition(|child| model.is_partnered_on_layer(layer, *child));
    let left_partner_count = partnered.len().div_ceil(2);
    partnered[..left_partner_count]
        .iter()
        .chain(&unpartnered)
        .chain(&partnered[left_partner_count..])
        .copied()
        .collect()
}

/// Reserve all sibling blocks before singles are considered. This is the
/// layout's structural priority: unrelated occurrences cannot split a family
/// or trade their own target error against the family's parent-center error.
/// Blocks are ordered by their parent anchors, with BFS rank and semantic IDs
/// supplying canonical subtree-swap tie breaks.
fn reserve_family_blocks(units: &mut [FamilyUnit], columns: &mut [Option<Pid>]) {
    if units.is_empty() {
        return;
    }
    units.sort_by(|first, second| {
        first
            .guide_target_twice
            .cmp(&second.guide_target_twice)
            .then_with(|| first.rank.cmp(&second.rank))
            .then_with(|| first.target_twice.cmp(&second.target_twice))
            .then_with(|| first.canonical_key.cmp(&second.canonical_key))
    });

    let width = columns.len();
    let count = units.len();
    let mut costs = vec![vec![None; width]; count];
    let mut predecessors = vec![vec![usize::MAX; width]; count];

    for start in available_block_starts(columns, units[0].members.len()) {
        costs[0][start] = Some(family_center_cost(start, &units[0]));
    }
    for unit_index in 1..count {
        let previous_size = units[unit_index - 1].members.len();
        let mut best = None::<((u128, u128), usize)>;
        for start in 0..width {
            if start >= previous_size {
                let previous_start = start - previous_size;
                if let Some(candidate) = costs[unit_index - 1][previous_start]
                    && best.is_none_or(|current| (candidate, previous_start) < current)
                {
                    best = Some((candidate, previous_start));
                }
            }
            if let Some((best_cost, previous_start)) = best
                && start + units[unit_index].members.len() <= width
                && columns[start..start + units[unit_index].members.len()]
                    .iter()
                    .all(Option::is_none)
            {
                let block_cost = family_center_cost(start, &units[unit_index]);
                costs[unit_index][start] = Some((
                    best_cost.0.saturating_add(block_cost.0),
                    best_cost.1.saturating_add(block_cost.1),
                ));
                predecessors[unit_index][start] = previous_start;
            }
        }
    }

    let mut start = (0..width)
        .filter_map(|start| costs[count - 1][start].map(|cost| (cost, start)))
        .min()
        .map(|(_, start)| start)
        .expect("all family blocks fit in a row that fits all occurrences");
    let mut starts = vec![0; count];
    for unit_index in (0..count).rev() {
        starts[unit_index] = start;
        if unit_index > 0 {
            start = predecessors[unit_index][start];
        }
    }
    for (unit, start) in units.iter().zip(starts) {
        for (offset, pid) in unit.members.iter().copied().enumerate() {
            debug_assert!(columns[start + offset].is_none());
            columns[start + offset] = Some(pid);
        }
    }
}

fn available_block_starts(
    columns: &[Option<Pid>],
    block_size: usize,
) -> impl Iterator<Item = usize> + '_ {
    (0..=columns.len() - block_size).filter(move |start| {
        columns[*start..*start + block_size]
            .iter()
            .all(Option::is_none)
    })
}

/// Family blocks are already indivisible before this score is consulted.
/// Coherent guide translation is the first numeric priority; exact alignment
/// with the already placed parents breaks guide ties, and unrelated singles
/// are placed only after all family starts have been fixed.
fn family_center_cost(start: usize, unit: &FamilyUnit) -> (u128, u128) {
    let center_twice = start
        .saturating_mul(2)
        .saturating_add(unit.members.len().saturating_sub(1));
    let difference = center_twice.abs_diff(unit.target_twice) as u128;
    let guide_difference = center_twice.abs_diff(unit.guide_target_twice) as u128;
    (
        guide_difference.saturating_mul(guide_difference),
        difference.saturating_mul(difference),
    )
}

fn parent_center_twice(
    relationship: &Relationship,
    layer: usize,
    placed_by_pid: &HashMap<Pid, Vec<PlacedOccurrence>>,
) -> Option<usize> {
    let mut columns = relationship
        .parents
        .iter()
        .flatten()
        .filter_map(|parent| nearest_prior_occurrence(*parent, layer, placed_by_pid))
        .map(|occurrence| occurrence.column)
        .collect::<Vec<_>>();
    columns.sort_unstable();
    match columns.as_slice() {
        [] => None,
        [column] => Some(column.saturating_mul(2)),
        [first, second, ..] => Some(first.saturating_add(*second)),
    }
}

fn nearest_anchor_twice(
    model: &Model<'_>,
    pid: Pid,
    layer: usize,
    placed_by_pid: &HashMap<Pid, Vec<PlacedOccurrence>>,
    fallback: usize,
) -> usize {
    let mut best =
        nearest_prior_occurrence(pid, layer.saturating_add(1), placed_by_pid).map(|occurrence| {
            (
                layer.abs_diff(occurrence.layer),
                0,
                model.rank(pid),
                pid,
                occurrence.column,
            )
        });
    for relationship_index in model.memberships.get(&pid).into_iter().flatten() {
        for relative in &model.relationship_members[*relationship_index] {
            if *relative == pid {
                continue;
            }
            if let Some(occurrence) =
                nearest_prior_occurrence(*relative, layer.saturating_add(1), placed_by_pid)
            {
                let candidate = (
                    layer.abs_diff(occurrence.layer),
                    1,
                    model.rank(*relative),
                    *relative,
                    occurrence.column,
                );
                if best.is_none_or(|current| candidate < current) {
                    best = Some(candidate);
                }
            }
        }
    }
    best.map_or(fallback, |candidate| candidate.4.saturating_mul(2))
}

fn nearest_prior_occurrence(
    pid: Pid,
    before_layer: usize,
    placed_by_pid: &HashMap<Pid, Vec<PlacedOccurrence>>,
) -> Option<PlacedOccurrence> {
    placed_by_pid.get(&pid).and_then(|occurrences| {
        occurrences
            .iter()
            .copied()
            .filter(|occurrence| occurrence.layer < before_layer)
            .max_by_key(|occurrence| (occurrence.layer, std::cmp::Reverse(occurrence.column)))
    })
}

fn canonical_relationship_cmp(first: &Relationship, second: &Relationship) -> std::cmp::Ordering {
    let mut first_parents = first.parents.iter().flatten().copied().collect::<Vec<_>>();
    let mut second_parents = second.parents.iter().flatten().copied().collect::<Vec<_>>();
    let mut first_children = first.children.clone();
    let mut second_children = second.children.clone();
    first_parents.sort_unstable();
    second_parents.sort_unstable();
    first_children.sort_unstable();
    second_children.sort_unstable();
    first_parents
        .cmp(&second_parents)
        .then_with(|| first_children.cmp(&second_children))
        .then_with(|| first.id.0.cmp(&second.id.0))
}

fn take_occurrence(remaining: &mut HashMap<Pid, usize>, pid: Pid) {
    let count = remaining
        .get_mut(&pid)
        .expect("family/root occurrence must be present in its layer");
    *count -= 1;
    if *count == 0 {
        remaining.remove(&pid);
    }
}

fn record_placement(
    placed_by_pid: &mut HashMap<Pid, Vec<PlacedOccurrence>>,
    pid: Pid,
    layer: usize,
    column: usize,
) {
    placed_by_pid
        .entry(pid)
        .or_default()
        .push(PlacedOccurrence { layer, column });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;

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

    fn run(layers: &[Vec<Pid>], relationships: &[Relationship]) -> PersonIndexOutput {
        let mut relationship_layers = vec![Vec::new(); layers.len()];
        for relationship in relationships {
            for layer in 0..layers.len() {
                let parents_present = layer > 0
                    && relationship
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

    fn column_on_layer(output: &PersonIndexOutput, layer: usize, pid: Pid) -> usize {
        output.person_indices[layer]
            .iter()
            .find(|person| person.pid == pid)
            .expect("person occurrence")
            .index
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

    fn placements(output: &PersonIndexOutput) -> Vec<Vec<(Pid, usize)>> {
        output
            .person_indices
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .map(|person| (person.pid, person.index))
                    .collect()
            })
            .collect()
    }

    #[test]
    fn chooses_the_canonical_oldest_person_as_centered_root() {
        let layers = vec![vec![pid(9), pid(1), pid(5)], vec![pid(2)]];
        let output = run(&layers, &[relationship(10, &[1, 2])]);

        assert_eq!(column_on_layer(&output, 0, pid(1)), 1);
    }

    #[test]
    fn semantic_family_bfs_defines_the_expansion_frontier() {
        let layers = vec![vec![pid(9), pid(1)], vec![pid(2)], vec![pid(3)]];
        let relationships = vec![relationship(10, &[1, 2]), relationship(11, &[2, 3])];
        let mut relationship_layers = vec![Vec::new(); layers.len()];
        relationship_layers[1].push(RelationshipId(10));
        relationship_layers[2].push(RelationshipId(11));
        let input = PersonIndexInput {
            person_layers: &layers,
            relationship_layers: &relationship_layers,
            relationships: &relationships,
            layout_algorithm: LayoutAlgorithm::KinshipExpansion,
        };
        let model = Model::new(&input, pid(1), 3, 0);

        assert_eq!(model.rank(pid(1)), 0);
        assert_eq!(model.rank(pid(2)), 1);
        assert_eq!(model.rank(pid(3)), 2);
        assert_eq!(model.rank(pid(9)), UNREACHABLE_RANK);
    }

    #[test]
    fn source_sibling_block_remains_contiguous_around_the_root_when_it_fits() {
        let layers = vec![vec![pid(9), pid(3), pid(1), pid(2)]];
        let relationships = vec![Relationship {
            id: RelationshipId(10),
            parents: [None, None],
            children: vec![pid(3), pid(1), pid(2)],
        }];
        let relationship_layers = vec![vec![RelationshipId(10)]];
        let output = get_person_indices(PersonIndexInput {
            person_layers: &layers,
            relationship_layers: &relationship_layers,
            relationships: &relationships,
            layout_algorithm: LayoutAlgorithm::KinshipExpansion,
        });
        let mut sibling_columns =
            [pid(1), pid(2), pid(3)].map(|person| column_on_layer(&output, 0, person));
        sibling_columns.sort_unstable();

        assert_eq!(column_on_layer(&output, 0, pid(1)), 2);
        assert!(
            sibling_columns
                .windows(2)
                .all(|pair| pair[1] == pair[0] + 1)
        );
        assert!(column_on_layer(&output, 0, pid(9)) < sibling_columns[0]);
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
    fn repeated_people_stay_on_every_layer_and_use_the_nearest_occurrence() {
        let layers = vec![
            vec![pid(1), pid(2)],
            vec![pid(2), pid(3)],
            vec![pid(2), pid(4)],
        ];
        let relationships = vec![relationship(10, &[1, 3]), relationship(11, &[3, 4])];
        let first = run(&layers, &relationships);
        let second = run(&layers, &relationships);

        let repeated_columns = first
            .person_indices
            .iter()
            .map(|layer| {
                layer
                    .iter()
                    .find(|person| person.pid == pid(2))
                    .expect("repeated occurrence")
                    .index
            })
            .collect::<Vec<_>>();
        assert_eq!(repeated_columns.len(), 3);
        assert!(repeated_columns[1].abs_diff(repeated_columns[2]) <= 1);
        assert_eq!(placements(&first), placements(&second));
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
    }

    #[test]
    fn unrelated_competitor_cannot_split_or_shift_an_isolated_sibling_group() {
        let layers = vec![vec![pid(1), pid(2), pid(90)], vec![pid(3), pid(99), pid(4)]];
        let relationships = vec![Relationship {
            id: RelationshipId(10),
            parents: [Some(pid(1)), Some(pid(2))],
            children: vec![pid(4), pid(3)],
        }];
        let output = run(&layers, &relationships);
        let parent_center_twice =
            column_on_layer(&output, 0, pid(1)) + column_on_layer(&output, 0, pid(2));
        let child_columns = [
            column_on_layer(&output, 1, pid(3)),
            column_on_layer(&output, 1, pid(4)),
        ];

        assert_eq!(child_columns[0].abs_diff(child_columns[1]), 1);
        assert_eq!(
            child_columns.into_iter().sum::<usize>(),
            parent_center_twice
        );
        let competitor = column_on_layer(&output, 1, pid(99));
        let minimum = *child_columns.iter().min().expect("child column");
        let maximum = *child_columns.iter().max().expect("child column");
        assert!(competitor < minimum || competitor > maximum);
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

        assert_eq!(
            column_on_layer(&output, 0, pid(1)) + column_on_layer(&output, 0, pid(2)),
            column_on_layer(&output, 1, pid(3)) + column_on_layer(&output, 1, pid(4))
        );
    }

    #[test]
    fn partnered_siblings_are_outside_after_source_permutations() {
        let layers = vec![
            vec![pid(2), pid(1)],
            vec![pid(8), pid(4), pid(7), pid(6), pid(3), pid(5)],
            vec![pid(10), pid(9)],
        ];
        let relationships = vec![
            Relationship {
                id: RelationshipId(12),
                parents: [Some(pid(6)), Some(pid(8))],
                children: vec![pid(10)],
            },
            Relationship {
                id: RelationshipId(10),
                parents: [Some(pid(2)), Some(pid(1))],
                children: vec![pid(6), pid(4), pid(3), pid(5)],
            },
            Relationship {
                id: RelationshipId(11),
                parents: [Some(pid(7)), Some(pid(3))],
                children: vec![pid(9)],
            },
        ];
        let output = run(&layers, &relationships);
        let partnered = [
            column_on_layer(&output, 1, pid(3)),
            column_on_layer(&output, 1, pid(6)),
        ];
        let outside = [
            *partnered.iter().min().expect("partnered child"),
            *partnered.iter().max().expect("partnered child"),
        ];

        for unpartnered in [pid(4), pid(5)] {
            let column = column_on_layer(&output, 1, unpartnered);
            assert!(outside[0] < column && column < outside[1]);
        }
    }

    #[test]
    fn canonical_layout_ignores_layer_relationship_and_member_source_order() {
        let layers = vec![
            vec![pid(9), pid(2), pid(1)],
            vec![pid(7), pid(4), pid(3), pid(8)],
            vec![pid(6), pid(5)],
        ];
        let relationships = vec![
            Relationship {
                id: RelationshipId(11),
                parents: [Some(pid(4)), None],
                children: vec![pid(6), pid(5)],
            },
            Relationship {
                id: RelationshipId(10),
                parents: [Some(pid(2)), Some(pid(1))],
                children: vec![pid(4), pid(3)],
            },
        ];
        let mut permuted_layers = layers.clone();
        for layer in &mut permuted_layers {
            layer.reverse();
        }
        let mut permuted_relationships = relationships.clone();
        permuted_relationships.reverse();
        for relationship in &mut permuted_relationships {
            relationship.parents.reverse();
            relationship.children.reverse();
        }

        let first = run(&layers, &relationships);
        let second = run(&permuted_layers, &permuted_relationships);
        assert_eq!(placements(&first), placements(&second));
    }

    #[test]
    fn got_obeys_root_family_sibling_and_occurrence_invariants() {
        use baumstamm_lib::{FamilyTree, graph::Graph};
        let tree = FamilyTree::try_from(include_str!("../../../examples/got/got.json"))
            .expect("valid GOT example");
        let relationships = tree.get_relationships();
        let graph = Graph::new(relationships).cut();
        let relationship_layers = graph.layers();
        let person_layers = graph.person_layers(relationships);
        let input = PersonIndexInput {
            person_layers: &person_layers,
            relationship_layers: &relationship_layers,
            relationships,
            layout_algorithm: LayoutAlgorithm::KinshipExpansion,
        };
        let output = get_person_indices(input);
        let widest = person_layers.iter().map(Vec::len).max().unwrap();
        let root = *person_layers[0].iter().min().expect("oldest person");
        let model = Model::new(&input, root, output.row_length, 0);

        assert_eq!(output.row_length, widest + widest.div_ceil(8));
        assert_eq!(
            column_on_layer(&output, 0, root),
            (output.row_length - 1) / 2
        );
        for (source, placed) in person_layers.iter().zip(&output.person_indices) {
            let mut expected = source.clone();
            let mut actual = placed.iter().map(|person| person.pid).collect::<Vec<_>>();
            expected.sort_unstable();
            actual.sort_unstable();
            assert_eq!(actual, expected, "every occurrence must be preserved");
            assert_eq!(
                placed
                    .iter()
                    .map(|person| person.index)
                    .collect::<HashSet<_>>()
                    .len(),
                placed.len(),
                "a generation must be collision-free"
            );
        }

        for (layer, relationship_ids) in relationship_layers.iter().enumerate().skip(1) {
            for relationship_id in relationship_ids {
                let relationship = relationships
                    .iter()
                    .find(|relationship| relationship.id == *relationship_id)
                    .expect("indexed relationship");
                let mut child_columns = relationship
                    .children
                    .iter()
                    .map(|child| column_on_layer(&output, layer, *child))
                    .collect::<Vec<_>>();
                child_columns.sort_unstable();
                assert!(
                    child_columns.windows(2).all(|pair| pair[1] == pair[0] + 1),
                    "rendered sibling blocks must remain contiguous"
                );

                let children = &relationship.children;
                if children.len() > 2 {
                    let (partnered, unpartnered): (Vec<_>, Vec<_>) = children
                        .iter()
                        .copied()
                        .partition(|child| model.is_partnered_on_layer(layer, *child));
                    if !partnered.is_empty() && !unpartnered.is_empty() {
                        let middle_min = unpartnered
                            .iter()
                            .map(|child| column_on_layer(&output, layer, *child))
                            .min()
                            .expect("unpartnered child");
                        let middle_max = unpartnered
                            .iter()
                            .map(|child| column_on_layer(&output, layer, *child))
                            .max()
                            .expect("unpartnered child");
                        assert!(partnered.iter().all(|child| {
                            let column = column_on_layer(&output, layer, *child);
                            column < middle_min || column > middle_max
                        }));
                    }
                }
            }
        }
    }
}
