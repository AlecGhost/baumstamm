use super::{PersonIndexInput, PersonIndexOutput};
use crate::indices::PersonIndex;
use baumstamm_lib::PersonId;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

type Pid = PersonId;

const REPEATED_PERSON_WEIGHT: u128 = 4;
const PARENT_CHILD_WEIGHT: u128 = 12;
const SPOUSE_WEIGHT: u128 = 16;

// These limits keep the combinatorial search predictable. Each round may keep
// a temporarily worse layout, which permits two-step staging moves, while the
// best strict objective seen over the entire search is returned.
const BEAM_WIDTH: usize = 2;
const SEARCH_ROUNDS: usize = 4;
const MAX_LAYER_PROPOSALS: usize = 16;
const MAX_BLOCK_PROPOSALS: usize = 32;
const MAX_SUBTREE_PAIR_PROPOSALS: usize = 24;
const BARYCENTRIC_SWEEPS: usize = 4;

/// Places fixed-generation occurrences by optimizing the horizontal segments
/// produced by the renderer. Generation/y coordinates never change.
///
/// The objective is lexicographic: maximum simultaneous point occupation, a
/// descending occupation histogram, pair-overlap area, horizontal length, and
/// parent/child center alignment. A canonical occurrence coordinate key breaks
/// remaining ties and makes source layer order irrelevant.
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

    let model = Model::new(&input);
    let widest_allowed = widest_layer
        .checked_add(widest_layer.div_ceil(8))
        .expect("connection layout width exceeds addressable memory");
    let mut best: Option<ScoredLayout> = None;

    for row_length in widest_layer..=widest_allowed {
        let candidate = optimize(&model, row_length);
        if best.as_ref().is_none_or(|current| {
            (
                &candidate.score,
                candidate.row_length,
                &candidate.coordinates,
            ) < (&current.score, current.row_length, &current.coordinates)
        }) {
            best = Some(candidate);
        }
    }

    let best = best.expect("a non-empty compact width band");
    PersonIndexOutput {
        person_indices: model.to_person_indices(&best.layout),
        row_length: best.row_length,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    max_occupation: u128,
    /// Counts of occupied points, ordered from the model's maximum possible
    /// occupation down to two. Occupation one is represented by length.
    occupation_histogram: Vec<u128>,
    pair_overlap_area: u128,
    length: u128,
    family_alignment: u128,
}

#[derive(Clone)]
struct ScoredLayout {
    layout: Vec<usize>,
    score: Score,
    row_length: usize,
    coordinates: Vec<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct OccurrenceKey {
    pid: Pid,
    layer: usize,
    repeat_ordinal: usize,
}

#[derive(Clone, Copy)]
struct Occurrence {
    key: OccurrenceKey,
}

#[derive(Clone)]
struct RenderedRelationship {
    parents: [Option<usize>; 2],
    children: Vec<usize>,
    has_declared_children: bool,
}

#[derive(Clone)]
struct FamilyBlock {
    people: Vec<usize>,
}

#[derive(Clone)]
struct Subtree {
    root_layer: usize,
    pids: BTreeSet<Pid>,
}

struct SubtreeRoot {
    layer: usize,
    parents: [Option<Pid>; 2],
    children: Vec<Pid>,
}

struct Model {
    occurrences: Vec<Occurrence>,
    layers: Vec<Vec<usize>>,
    rows: Vec<Vec<RenderedRelationship>>,
    neighbours: Vec<Vec<(usize, u128)>>,
    family_blocks: Vec<FamilyBlock>,
    subtrees: Vec<Subtree>,
    max_channel_lines: usize,
}

impl Model {
    fn new(input: &PersonIndexInput<'_>) -> Self {
        let mut occurrences = Vec::new();
        let mut layers = Vec::with_capacity(input.person_layers.len());
        let mut first_at = BTreeMap::<(usize, Pid), usize>::new();
        let mut by_pid = BTreeMap::<Pid, Vec<usize>>::new();

        for (layer, source_people) in input.person_layers.iter().enumerate() {
            let mut sorted = source_people.clone();
            sorted.sort();
            let mut ordinals = BTreeMap::<Pid, usize>::new();
            let mut layer_occurrences = Vec::with_capacity(sorted.len());
            for pid in sorted {
                let repeat_ordinal = ordinals.entry(pid).or_default();
                let occurrence = occurrences.len();
                occurrences.push(Occurrence {
                    key: OccurrenceKey {
                        pid,
                        layer,
                        repeat_ordinal: *repeat_ordinal,
                    },
                });
                *repeat_ordinal += 1;
                first_at.entry((layer, pid)).or_insert(occurrence);
                by_pid.entry(pid).or_default().push(occurrence);
                layer_occurrences.push(occurrence);
            }
            layers.push(layer_occurrences);
        }

        let relationships = input
            .relationships
            .iter()
            .map(|relationship| (relationship.id, relationship))
            .collect::<HashMap<_, _>>();
        let mut rows = Vec::with_capacity(input.relationship_layers.len());
        let mut family_blocks = Vec::new();
        let mut child_edges = BTreeMap::<Pid, BTreeSet<Pid>>::new();
        let mut subtree_roots = Vec::new();

        for (layer, relationship_ids) in input.relationship_layers.iter().enumerate() {
            let mut ids = relationship_ids.clone();
            ids.sort_by_key(|id| id.0);
            let mut row = Vec::with_capacity(ids.len());
            for relationship_id in ids {
                let relationship = relationships
                    .get(&relationship_id)
                    .expect("relationship layer references an unknown relationship");
                let parents = relationship.parents.map(|parent| {
                    parent.and_then(|pid| {
                        layer
                            .checked_sub(1)
                            .and_then(|parent_layer| first_at.get(&(parent_layer, pid)).copied())
                    })
                });
                let children = relationship
                    .children
                    .iter()
                    .filter_map(|pid| first_at.get(&(layer, *pid)).copied())
                    .collect::<Vec<_>>();
                let complete_children =
                    (children.len() == relationship.children.len()).then_some(children);

                let parent_people = parents.iter().flatten().copied().collect::<Vec<_>>();
                if parent_people.len() > 1 {
                    family_blocks.push(FamilyBlock {
                        people: parent_people,
                    });
                }
                if let Some(children) = &complete_children
                    && children.len() > 1
                {
                    family_blocks.push(FamilyBlock {
                        people: children.clone(),
                    });
                }
                for parent in relationship.parents.iter().flatten() {
                    child_edges
                        .entry(*parent)
                        .or_default()
                        .extend(relationship.children.iter().copied());
                }
                subtree_roots.push(SubtreeRoot {
                    layer: layer.saturating_sub(1),
                    parents: relationship.parents,
                    children: relationship.children.clone(),
                });
                row.push(RenderedRelationship {
                    parents,
                    children: complete_children.unwrap_or_default(),
                    has_declared_children: !relationship.children.is_empty(),
                });
            }
            rows.push(row);
        }

        let mut neighbours = vec![Vec::new(); occurrences.len()];
        for same_person in by_pid.values() {
            for first in 0..same_person.len() {
                for second in first + 1..same_person.len() {
                    add_neighbour(
                        &mut neighbours,
                        same_person[first],
                        same_person[second],
                        REPEATED_PERSON_WEIGHT,
                    );
                }
            }
        }
        for row in &rows {
            for family in row {
                let parents = family.parents.iter().flatten().copied().collect::<Vec<_>>();
                for first in 0..parents.len() {
                    for second in first + 1..parents.len() {
                        add_neighbour(
                            &mut neighbours,
                            parents[first],
                            parents[second],
                            SPOUSE_WEIGHT,
                        );
                    }
                }
                for parent in &parents {
                    for child in &family.children {
                        add_neighbour(&mut neighbours, *parent, *child, PARENT_CHILD_WEIGHT);
                    }
                }
            }
        }
        for adjacent in &mut neighbours {
            adjacent.sort_by_key(|(person, weight)| (*person, *weight));
        }

        family_blocks.sort_by(|first, second| first.people.cmp(&second.people));
        family_blocks.dedup_by(|first, second| first.people == second.people);
        let subtrees = build_subtrees(&subtree_roots, &child_edges);
        let max_channel_lines = rows.iter().map(Vec::len).max().unwrap_or_default();

        Self {
            occurrences,
            layers,
            rows,
            neighbours,
            family_blocks,
            subtrees,
            max_channel_lines,
        }
    }

    fn centered_layout(&self, row_length: usize) -> Vec<usize> {
        let mut layout = vec![0; self.occurrences.len()];
        for people in &self.layers {
            let start = (row_length - people.len()) / 2;
            for (offset, person) in people.iter().enumerate() {
                layout[*person] = start + offset;
            }
        }
        layout
    }

    fn structural_layout(&self, row_length: usize) -> Vec<usize> {
        let mut layout = self.centered_layout(row_length);
        for sweep in 0..BARYCENTRIC_SWEEPS {
            let layer_order = if sweep % 2 == 0 {
                (0..self.layers.len()).collect::<Vec<_>>()
            } else {
                (0..self.layers.len()).rev().collect::<Vec<_>>()
            };
            for layer in layer_order {
                let mut people = self.layers[layer].clone();
                people.sort_by(|first, second| {
                    target_fraction(self, &layout, *first)
                        .cmp(&target_fraction(self, &layout, *second))
                        .then_with(|| {
                            self.occurrences[*first]
                                .key
                                .cmp(&self.occurrences[*second].key)
                        })
                });
                let start = (row_length - people.len()) / 2;
                for (offset, person) in people.into_iter().enumerate() {
                    layout[person] = start + offset;
                }
            }
        }
        layout
    }

    fn to_person_indices(&self, layout: &[usize]) -> Vec<Vec<PersonIndex>> {
        self.layers
            .iter()
            .map(|people| {
                people
                    .iter()
                    .map(|person| PersonIndex {
                        pid: self.occurrences[*person].key.pid,
                        index: layout[*person],
                    })
                    .collect()
            })
            .collect()
    }
}

fn build_subtrees(
    roots: &[SubtreeRoot],
    child_edges: &BTreeMap<Pid, BTreeSet<Pid>>,
) -> Vec<Subtree> {
    let mut subtrees = Vec::new();
    for root in roots {
        let mut pids = root
            .parents
            .iter()
            .flatten()
            .chain(&root.children)
            .copied()
            .collect::<BTreeSet<_>>();
        let mut queue = pids.iter().copied().collect::<Vec<_>>();
        while let Some(pid) = queue.pop() {
            if let Some(children) = child_edges.get(&pid) {
                for child in children {
                    if pids.insert(*child) {
                        queue.push(*child);
                    }
                }
            }
        }
        if pids.len() > 1 {
            subtrees.push(Subtree {
                root_layer: root.layer,
                pids,
            });
        }
    }
    subtrees.sort_by(|first, second| {
        (first.root_layer, &first.pids).cmp(&(second.root_layer, &second.pids))
    });
    subtrees.dedup_by(|first, second| {
        first.root_layer == second.root_layer && first.pids == second.pids
    });
    subtrees
}

fn add_neighbour(neighbours: &mut [Vec<(usize, u128)>], first: usize, second: usize, weight: u128) {
    if first == second {
        return;
    }
    neighbours[first].push((second, weight));
    neighbours[second].push((first, weight));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TargetFraction {
    numerator: u128,
    denominator: u128,
}

impl Ord for TargetFraction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.numerator
            .checked_mul(other.denominator)
            .expect("target fraction fits in u128")
            .cmp(
                &other
                    .numerator
                    .checked_mul(self.denominator)
                    .expect("target fraction fits in u128"),
            )
    }
}

impl PartialOrd for TargetFraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn target_fraction(model: &Model, layout: &[usize], person: usize) -> TargetFraction {
    let (numerator, denominator) = model.neighbours[person].iter().fold(
        (0_u128, 0_u128),
        |(numerator, denominator), (other, weight)| {
            (
                numerator
                    .checked_add(
                        (*weight)
                            .checked_mul(layout[*other] as u128)
                            .expect("weighted target fits in u128"),
                    )
                    .expect("weighted target sum fits in u128"),
                denominator
                    .checked_add(*weight)
                    .expect("target weight sum fits in u128"),
            )
        },
    );
    if denominator == 0 {
        TargetFraction {
            numerator: layout[person] as u128,
            denominator: 1,
        }
    } else {
        TargetFraction {
            numerator,
            denominator,
        }
    }
}

fn optimize(model: &Model, row_length: usize) -> ScoredLayout {
    let centered = model.centered_layout(row_length);
    let structural = model.structural_layout(row_length);
    let seeds = [
        centered.clone(),
        reflect(&centered, row_length),
        structural.clone(),
        reflect(&structural, row_length),
    ];
    let mut score_cache = BTreeMap::<Vec<usize>, Score>::new();
    let mut visited = BTreeSet::<Vec<usize>>::new();
    let mut frontier = Vec::new();
    let mut best: Option<ScoredLayout> = None;

    for seed in seeds {
        if !visited.insert(seed.clone()) {
            continue;
        }
        let scored = scored_layout(model, seed, row_length, &mut score_cache);
        update_best(&mut best, &scored);
        frontier.push(scored);
    }
    frontier.sort_by(scored_order);
    frontier.truncate(BEAM_WIDTH);

    for _ in 0..SEARCH_ROUNDS {
        let mut next = Vec::new();
        for state in &frontier {
            for proposal in proposals(model, &state.layout, row_length) {
                if !visited.insert(proposal.clone()) {
                    continue;
                }
                let scored = scored_layout(model, proposal, row_length, &mut score_cache);
                update_best(&mut best, &scored);
                next.push(scored);
            }
        }
        if next.is_empty() {
            break;
        }
        next.sort_by(scored_order);
        next.truncate(BEAM_WIDTH);
        frontier = next;
    }

    best.expect("at least one deterministic seed")
}

fn scored_layout(
    model: &Model,
    layout: Vec<usize>,
    row_length: usize,
    cache: &mut BTreeMap<Vec<usize>, Score>,
) -> ScoredLayout {
    let score = cache
        .entry(layout.clone())
        .or_insert_with(|| score(model, &layout))
        .clone();
    ScoredLayout {
        coordinates: layout.clone(),
        layout,
        score,
        row_length,
    }
}

fn scored_order(first: &ScoredLayout, second: &ScoredLayout) -> Ordering {
    (&first.score, &first.coordinates).cmp(&(&second.score, &second.coordinates))
}

fn update_best(best: &mut Option<ScoredLayout>, candidate: &ScoredLayout) {
    if best.as_ref().is_none_or(|current| {
        (&candidate.score, &candidate.coordinates) < (&current.score, &current.coordinates)
    }) {
        *best = Some(candidate.clone());
    }
}

fn reflect(layout: &[usize], row_length: usize) -> Vec<usize> {
    layout
        .iter()
        .map(|column| row_length - 1 - column)
        .collect()
}

fn proposals(model: &Model, layout: &[usize], row_length: usize) -> Vec<Vec<usize>> {
    let mut proposals = Vec::new();
    let mut unique = BTreeSet::new();

    for people in &model.layers {
        let mut layer_proposals = Vec::new();
        let mut ordered = people.clone();
        ordered.sort_by_key(|person| (layout[*person], model.occurrences[*person].key));

        for adjacent in ordered.windows(2) {
            let mut candidate = layout.to_vec();
            candidate.swap(adjacent[0], adjacent[1]);
            push_unique(&mut layer_proposals, &mut unique, candidate);
        }

        let occupied = ordered
            .iter()
            .map(|person| (layout[*person], *person))
            .collect::<BTreeMap<_, _>>();
        for person in &ordered {
            let target = target_column(model, layout, *person, row_length);
            if let Some(other) = occupied.get(&target) {
                if other != person {
                    let mut candidate = layout.to_vec();
                    candidate.swap(*person, *other);
                    push_unique(&mut layer_proposals, &mut unique, candidate);
                }
            } else {
                let mut candidate = layout.to_vec();
                candidate[*person] = target;
                push_unique(&mut layer_proposals, &mut unique, candidate);
            }
        }

        if ordered.len() > 2 {
            let mut candidate = layout.to_vec();
            let mut columns = ordered
                .iter()
                .map(|person| layout[*person])
                .collect::<Vec<_>>();
            columns.sort_unstable();
            for (person, column) in ordered.iter().zip(columns.into_iter().rev()) {
                candidate[*person] = column;
            }
            push_unique(&mut layer_proposals, &mut unique, candidate);
        }

        layer_proposals.sort();
        layer_proposals.truncate(MAX_LAYER_PROPOSALS);
        proposals.extend(layer_proposals);
    }

    let mut block_proposals = Vec::new();
    for block in &model.family_blocks {
        if block.people.len() < 2 {
            continue;
        }
        let layer = model.occurrences[block.people[0]].key.layer;
        let layer_people = &model.layers[layer];
        let block_set = block.people.iter().copied().collect::<BTreeSet<_>>();
        for insertion in [0, layer_people.len() / 2, layer_people.len()] {
            if let Some(candidate) =
                relocate_block(model, layout, layer_people, &block_set, insertion, false)
            {
                push_unique(&mut block_proposals, &mut unique, candidate);
            }
        }
        if let Some(candidate) = relocate_block(
            model,
            layout,
            layer_people,
            &block_set,
            layer_people.len() / 2,
            true,
        ) {
            push_unique(&mut block_proposals, &mut unique, candidate);
        }
    }
    block_proposals.sort();
    block_proposals.truncate(MAX_BLOCK_PROPOSALS);
    proposals.extend(block_proposals);

    let mut subtree_proposals = Vec::new();
    'pairs: for first in 0..model.subtrees.len() {
        for second in first + 1..model.subtrees.len() {
            let left = &model.subtrees[first];
            let right = &model.subtrees[second];
            if left.root_layer != right.root_layer || !left.pids.is_disjoint(&right.pids) {
                continue;
            }
            for reverse in [false, true] {
                if let Some(candidate) = reorder_subtrees(model, layout, left, right, reverse) {
                    push_unique(&mut subtree_proposals, &mut unique, candidate);
                }
            }
            if subtree_proposals.len() >= MAX_SUBTREE_PAIR_PROPOSALS {
                break 'pairs;
            }
        }
    }
    subtree_proposals.sort();
    subtree_proposals.truncate(MAX_SUBTREE_PAIR_PROPOSALS);
    proposals.extend(subtree_proposals);
    push_unique(&mut proposals, &mut unique, reflect(layout, row_length));
    proposals
}

fn push_unique(
    proposals: &mut Vec<Vec<usize>>,
    unique: &mut BTreeSet<Vec<usize>>,
    candidate: Vec<usize>,
) {
    if unique.insert(candidate.clone()) {
        proposals.push(candidate);
    }
}

fn target_column(model: &Model, layout: &[usize], person: usize, row_length: usize) -> usize {
    let target = target_fraction(model, layout, person);
    let rounded = target
        .numerator
        .checked_add(target.denominator / 2)
        .expect("rounded target fits in u128")
        / target.denominator;
    usize::try_from(rounded)
        .expect("target column fits in usize")
        .min(row_length - 1)
}

fn relocate_block(
    model: &Model,
    layout: &[usize],
    layer_people: &[usize],
    block: &BTreeSet<usize>,
    insertion: usize,
    reverse: bool,
) -> Option<Vec<usize>> {
    let mut ordered = layer_people.to_vec();
    ordered.sort_by_key(|person| (layout[*person], model.occurrences[*person].key));
    let mut selected = ordered
        .iter()
        .filter(|person| block.contains(person))
        .copied()
        .collect::<Vec<_>>();
    if selected.len() < 2 {
        return None;
    }
    if reverse {
        selected.reverse();
    }
    let mut remaining = ordered
        .iter()
        .filter(|person| !block.contains(person))
        .copied()
        .collect::<Vec<_>>();
    let insertion = insertion.min(remaining.len());
    remaining.splice(insertion..insertion, selected);
    let mut columns = ordered
        .iter()
        .map(|person| layout[*person])
        .collect::<Vec<_>>();
    columns.sort_unstable();
    let mut candidate = layout.to_vec();
    for (person, column) in remaining.into_iter().zip(columns) {
        candidate[person] = column;
    }
    (candidate != layout).then_some(candidate)
}

fn reorder_subtrees(
    model: &Model,
    layout: &[usize],
    first: &Subtree,
    second: &Subtree,
    reverse: bool,
) -> Option<Vec<usize>> {
    let mut candidate = layout.to_vec();
    let mut changed = false;
    for (layer, layer_people) in model.layers.iter().enumerate() {
        if layer < first.root_layer {
            continue;
        }
        let mut first_people = layer_people
            .iter()
            .filter(|person| first.pids.contains(&model.occurrences[**person].key.pid))
            .copied()
            .collect::<Vec<_>>();
        let mut second_people = layer_people
            .iter()
            .filter(|person| second.pids.contains(&model.occurrences[**person].key.pid))
            .copied()
            .collect::<Vec<_>>();
        if first_people.is_empty() || second_people.is_empty() {
            continue;
        }
        first_people.sort_by_key(|person| (layout[*person], model.occurrences[*person].key));
        second_people.sort_by_key(|person| (layout[*person], model.occurrences[*person].key));
        if reverse {
            first_people.reverse();
            second_people.reverse();
        }
        let mut slots = first_people
            .iter()
            .chain(&second_people)
            .map(|person| layout[*person])
            .collect::<Vec<_>>();
        slots.sort_unstable();
        let people = second_people.into_iter().chain(first_people);
        for (person, slot) in people.zip(slots) {
            changed |= candidate[person] != slot;
            candidate[person] = slot;
        }
    }
    changed.then_some(candidate)
}

fn score(model: &Model, layout: &[usize]) -> Score {
    let mut histogram = vec![0_u128; model.max_channel_lines.saturating_sub(1)];
    let mut max_occupation = 0_u128;
    let mut pair_overlap_area = 0_u128;
    let mut length = 0_u128;
    let mut family_alignment = 0_u128;

    for row in &model.rows {
        let mut parent_lines = Vec::new();
        let mut child_lines = Vec::new();
        for relationship in row {
            let parents = relationship
                .parents
                .iter()
                .flatten()
                .map(|person| layout[*person])
                .collect::<Vec<_>>();
            let crossing = if relationship.has_declared_children {
                crossing_point(&parents)
            } else {
                None
            };
            let children = relationship
                .children
                .iter()
                .map(|person| layout[*person])
                .collect::<Vec<_>>();

            if !parents.is_empty() && !children.is_empty() {
                family_alignment = family_alignment
                    .checked_add(center_twice(&parents).abs_diff(center_twice(&children)))
                    .expect("family alignment score fits in u128");
            }
            if let Some(line) = parent_interval(&parents, crossing) {
                length = length
                    .checked_add(line.1.abs_diff(line.0) as u128)
                    .expect("horizontal length fits in u128");
                parent_lines.push(line);
            }
            if let Some(line) = child_interval(&children, crossing) {
                length = length
                    .checked_add(line.1.abs_diff(line.0) as u128)
                    .expect("horizontal length fits in u128");
                child_lines.push(line);
            }
        }
        for channel in [&parent_lines, &child_lines] {
            score_channel(
                channel,
                &mut histogram,
                &mut max_occupation,
                &mut pair_overlap_area,
            );
        }
    }

    histogram.reverse();
    Score {
        max_occupation,
        occupation_histogram: histogram,
        pair_overlap_area,
        length,
        family_alignment,
    }
}

fn crossing_point(parents: &[usize]) -> Option<usize> {
    match parents {
        [] => None,
        [parent] => Some(*parent),
        [first, second, ..] => {
            let minimum = (*first).min(*second);
            Some(minimum + first.abs_diff(*second) / 2)
        }
    }
}

fn parent_interval(parents: &[usize], crossing: Option<usize>) -> Option<(usize, usize)> {
    match (parents, crossing) {
        ([first, second, ..], _) => Some(((*first).min(*second), (*first).max(*second))),
        ([parent], Some(crossing)) if *parent != crossing => {
            Some(((*parent).min(crossing), (*parent).max(crossing)))
        }
        _ => None,
    }
}

fn child_interval(children: &[usize], crossing: Option<usize>) -> Option<(usize, usize)> {
    let first = children.iter().min().copied()?;
    let last = children.iter().max().copied()?;
    let (start, end) = crossing.map_or((first, last), |crossing| {
        (first.min(crossing), last.max(crossing))
    });
    (start != end).then_some((start, end))
}

fn score_channel(
    lines: &[(usize, usize)],
    histogram: &mut [u128],
    max_occupation: &mut u128,
    pair_overlap_area: &mut u128,
) {
    let mut events = BTreeMap::<usize, i128>::new();
    for (start, end) in lines {
        *events.entry(*start).or_default() += 1;
        *events
            .entry(end.checked_add(1).expect("line endpoint fits in usize"))
            .or_default() -= 1;
    }
    let mut occupation = 0_i128;
    let mut previous = None;
    for (column, delta) in events {
        if let Some(previous) = previous {
            let point_count = (column - previous) as u128;
            if occupation > 0 && point_count > 0 {
                *max_occupation = (*max_occupation).max(occupation as u128);
            }
            if occupation >= 2 && point_count > 0 {
                let occupation = occupation as u128;
                let histogram_index =
                    usize::try_from(occupation - 2).expect("occupation count fits in usize");
                histogram[histogram_index] = histogram[histogram_index]
                    .checked_add(point_count)
                    .expect("occupation histogram fits in u128");
                let pairs = occupation
                    .checked_mul(occupation - 1)
                    .and_then(|value| value.checked_div(2))
                    .expect("pair occupation fits in u128");
                *pair_overlap_area = pair_overlap_area
                    .checked_add(
                        pairs
                            .checked_mul(point_count)
                            .expect("pair overlap span fits in u128"),
                    )
                    .expect("pair overlap area fits in u128");
            }
        }
        occupation += delta;
        debug_assert!(occupation >= 0);
        previous = Some(column);
    }
    debug_assert_eq!(occupation, 0);
}

fn center_twice(columns: &[usize]) -> u128 {
    let minimum = columns.iter().min().copied().unwrap_or_default() as u128;
    let maximum = columns.iter().max().copied().unwrap_or_default() as u128;
    minimum
        .checked_add(maximum)
        .expect("twice-center fits in u128")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use crate::{indices, lines};
    use baumstamm_lib::{Relationship, RelationshipId};

    fn pid(value: u128) -> PersonId {
        value.into()
    }

    fn relationship(id: u128, parents: [u128; 2], children: &[u128]) -> Relationship {
        Relationship {
            id: RelationshipId(id),
            parents: parents.map(pid).map(Some),
            children: children.iter().copied().map(pid).collect(),
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

    fn canonical_coordinates(output: &PersonIndexOutput) -> Vec<(usize, PersonId, usize, usize)> {
        output
            .person_indices
            .iter()
            .enumerate()
            .flat_map(|(layer, people)| {
                let mut ordinals = BTreeMap::<PersonId, usize>::new();
                people.iter().map(move |person| {
                    let ordinal = ordinals.entry(person.pid).or_default();
                    let key = (layer, person.pid, *ordinal, person.index);
                    *ordinal += 1;
                    key
                })
            })
            .collect()
    }

    fn renderer_reference_score(
        request: &PersonIndexInput<'_>,
        model: &Model,
        layout: &[usize],
        row_length: usize,
    ) -> Score {
        let person_indices = model.to_person_indices(layout);
        let relationship_indices = indices::get_rel_indices(
            request.relationship_layers,
            request.relationships,
            &person_indices,
        );
        let mut occupation_histogram = vec![0_u128; model.max_channel_lines.saturating_sub(1)];
        let mut max_occupation = 0_u128;
        let mut pair_overlap_area = 0_u128;
        let mut length = 0_u128;
        let mut family_alignment = 0_u128;
        for row in relationship_indices {
            for relationship in &row {
                let parents = relationship.get_parents();
                if !parents.is_empty() && !relationship.children.is_empty() {
                    family_alignment +=
                        center_twice(&parents).abs_diff(center_twice(&relationship.children));
                }
            }
            for channel in lines::create_horizontal(&row) {
                let mut occupation = vec![0_u128; row_length];
                for line in channel {
                    length += line.end.abs_diff(line.start) as u128;
                    for point in &mut occupation[line.start..=line.end] {
                        *point += 1;
                    }
                }
                for point in occupation {
                    max_occupation = max_occupation.max(point);
                    if point >= 2 {
                        occupation_histogram[(point - 2) as usize] += 1;
                        pair_overlap_area += point * (point - 1) / 2;
                    }
                }
            }
        }
        occupation_histogram.reverse();
        Score {
            max_occupation,
            occupation_histogram,
            pair_overlap_area,
            length,
            family_alignment,
        }
    }

    #[test]
    fn shuffled_source_order_has_canonical_output() {
        let people = vec![
            vec![pid(1), pid(3), pid(2), pid(4)],
            vec![pid(5), pid(7), pid(6), pid(8)],
        ];
        let shuffled = vec![
            vec![pid(4), pid(2), pid(3), pid(1)],
            vec![pid(8), pid(6), pid(7), pid(5)],
        ];
        let relationships = vec![
            relationship(10, [1, 2], &[5, 6]),
            relationship(11, [3, 4], &[7, 8]),
        ];
        let shuffled_relationships = vec![relationships[1].clone(), relationships[0].clone()];
        let relationship_layers = vec![vec![], vec![RelationshipId(11), RelationshipId(10)]];
        let shuffled_relationship_layers =
            vec![vec![], vec![RelationshipId(10), RelationshipId(11)]];

        let first = get_person_indices(input(&people, &relationship_layers, &relationships));
        let second = get_person_indices(input(
            &shuffled,
            &shuffled_relationship_layers,
            &shuffled_relationships,
        ));

        assert_eq!(first.row_length, second.row_length);
        assert_eq!(
            canonical_coordinates(&first),
            canonical_coordinates(&second)
        );
    }

    #[test]
    fn preindexed_score_matches_renderer_channels_and_endpoints() {
        let people = vec![
            vec![pid(1), pid(3), pid(2), pid(4)],
            vec![pid(5), pid(7), pid(6), pid(8)],
        ];
        let relationships = vec![
            relationship(10, [1, 2], &[5, 6]),
            relationship(11, [3, 4], &[7, 8]),
        ];
        let relationship_layers = vec![vec![], vec![RelationshipId(10), RelationshipId(11)]];
        let request = input(&people, &relationship_layers, &relationships);
        let model = Model::new(&request);
        let layouts = [model.centered_layout(5), model.structural_layout(5)];

        for layout in layouts {
            assert_eq!(
                score(&model, &layout),
                renderer_reference_score(&request, &model, &layout, 5)
            );
        }
    }

    #[test]
    fn structured_search_reorders_descendant_subtrees_coherently() {
        let people = vec![
            vec![pid(1), pid(2), pid(3), pid(4)],
            vec![pid(7), pid(8), pid(5), pid(6)],
            vec![pid(11), pid(12), pid(9), pid(10)],
        ];
        let relationships = vec![
            relationship(10, [1, 2], &[5, 6]),
            relationship(11, [3, 4], &[7, 8]),
            relationship(12, [5, 6], &[9, 10]),
            relationship(13, [7, 8], &[11, 12]),
        ];
        let relationship_layers = vec![
            vec![],
            vec![RelationshipId(10), RelationshipId(11)],
            vec![RelationshipId(12), RelationshipId(13)],
        ];

        let output = get_person_indices(input(&people, &relationship_layers, &relationships));
        let center = |layer: usize, pids: &[PersonId]| {
            let columns = output.person_indices[layer]
                .iter()
                .filter(|person| pids.contains(&person.pid))
                .map(|person| person.index)
                .collect::<Vec<_>>();
            center_twice(&columns)
        };
        let root_order = center(0, &[pid(1), pid(2)]).cmp(&center(0, &[pid(3), pid(4)]));

        assert_ne!(root_order, Ordering::Equal);
        assert_eq!(
            center(1, &[pid(5), pid(6)]).cmp(&center(1, &[pid(7), pid(8)])),
            root_order
        );
        assert_eq!(
            center(2, &[pid(9), pid(10)]).cmp(&center(2, &[pid(11), pid(12)])),
            root_order
        );
    }

    #[test]
    fn width_is_selected_from_the_compact_band() {
        let people = vec![vec![pid(1), pid(2), pid(3), pid(4), pid(5)], vec![pid(6)]];
        let relationship_layers = vec![vec![], vec![]];
        let output = get_person_indices(input(&people, &relationship_layers, &[]));

        assert!((5..=6).contains(&output.row_length));
        assert_eq!(
            output.row_length, 5,
            "equal objectives prefer narrower width"
        );
    }

    #[test]
    fn preserves_exact_layer_multisets_and_collisions_are_impossible() {
        let people = vec![
            vec![pid(3), pid(1), pid(3), pid(2)],
            vec![pid(5), pid(4), pid(1)],
        ];
        let relationship_layers = vec![vec![], vec![]];
        let output = get_person_indices(input(&people, &relationship_layers, &[]));

        for (source, result) in people.iter().zip(&output.person_indices) {
            let mut source = source.clone();
            source.sort();
            let mut actual = result.iter().map(|person| person.pid).collect::<Vec<_>>();
            actual.sort();
            let occupied = result
                .iter()
                .map(|person| person.index)
                .collect::<BTreeSet<_>>();
            assert_eq!(source, actual);
            assert_eq!(occupied.len(), result.len());
            assert!(occupied.iter().all(|column| *column < output.row_length));
        }
    }

    #[test]
    fn pointwise_peak_precedes_pair_overlap_and_length() {
        let people = vec![
            vec![pid(1), pid(2), pid(3), pid(4), pid(5), pid(6)],
            vec![pid(7), pid(8), pid(9), pid(10), pid(11), pid(12)],
        ];
        let relationships = vec![
            relationship(20, [1, 2], &[7, 8]),
            relationship(21, [3, 4], &[9, 10]),
            relationship(22, [5, 6], &[11, 12]),
        ];
        let relationship_layers = vec![
            vec![],
            vec![RelationshipId(20), RelationshipId(21), RelationshipId(22)],
        ];
        let model = Model::new(&input(&people, &relationship_layers, &relationships));
        let output = get_person_indices(input(&people, &relationship_layers, &relationships));
        let optimized = output
            .person_indices
            .iter()
            .flatten()
            .map(|person| person.index)
            .collect::<Vec<_>>();
        let optimized_score = score(&model, &optimized);

        assert!(optimized_score.max_occupation <= 2);
    }

    #[test]
    fn exhaustive_small_layout_confirms_returned_objective() {
        let people = vec![vec![pid(1), pid(2), pid(3)], vec![pid(4), pid(5), pid(6)]];
        let relationships = vec![
            relationship(10, [1, 2], &[4, 5]),
            relationship(11, [2, 3], &[5, 6]),
        ];
        let relationship_layers = vec![vec![], vec![RelationshipId(10), RelationshipId(11)]];
        let request = input(&people, &relationship_layers, &relationships);
        let model = Model::new(&request);
        let output = get_person_indices(request);
        assert_eq!(output.row_length, 3);
        let actual = output
            .person_indices
            .iter()
            .flatten()
            .map(|person| person.index)
            .collect::<Vec<_>>();
        let actual_score = score(&model, &actual);
        let permutations = [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ];
        let oracle = permutations
            .iter()
            .flat_map(|first| {
                permutations.iter().map(|second| {
                    let layout = first.iter().chain(second).copied().collect::<Vec<_>>();
                    (score(&model, &layout), layout)
                })
            })
            .min()
            .expect("finite oracle");

        assert_eq!((actual_score, actual), oracle);
    }
}
