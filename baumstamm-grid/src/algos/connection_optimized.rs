use super::{PersonIndexInput, PersonIndexOutput};
use crate::{
    indices::{self, PersonIndex},
    lines,
};

const MAX_EXTRA_COLUMNS: usize = 3;
const MAX_PASSES_PER_WIDTH: usize = 12;
const MAX_EVALUATIONS_PER_WIDTH: usize = 1_000;

/// Places fixed-layer person occurrences by minimizing the horizontal
/// connections produced by the same model that renders the grid.
///
/// Candidate layouts are ordered by pairwise horizontal-segment congestion
/// first and total horizontal length second. Width and stable occurrence
/// coordinates provide deterministic tie-breakers between equally good
/// layouts.
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

    let maximum_width = widest_layer.saturating_add(MAX_EXTRA_COLUMNS);
    let mut best: Option<Candidate> = None;
    for row_length in widest_layer..=maximum_width {
        let candidate = optimize_width(&input, row_length);
        if best
            .as_ref()
            .is_none_or(|current| candidate.rank() < current.rank())
        {
            best = Some(candidate);
        }
    }

    let best = best.expect("a non-empty layout has at least one candidate width");
    PersonIndexOutput {
        person_indices: best.person_indices,
        row_length: best.row_length,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    congestion: u64,
    length: u64,
}

struct Candidate {
    person_indices: Vec<Vec<PersonIndex>>,
    row_length: usize,
    score: Score,
}

impl Candidate {
    fn rank(&self) -> (Score, usize, Vec<usize>) {
        (
            self.score,
            self.row_length,
            coordinate_key(&self.person_indices),
        )
    }
}

fn optimize_width(input: &PersonIndexInput<'_>, row_length: usize) -> Candidate {
    let mut person_indices = centered_indices(input.person_layers, row_length);
    let mut current_score = score(input, &person_indices, row_length);
    let mut evaluations = 1;

    for _ in 0..MAX_PASSES_PER_WIDTH {
        let mut neighbourhood = Neighbourhood {
            input,
            row_length,
            current_score,
            best_move: None,
            evaluations: &mut evaluations,
        };

        // Adjacent swaps cheaply repair most interleaved families first.
        for layer in 0..person_indices.len() {
            for first in 0..person_indices[layer].len().saturating_sub(1) {
                neighbourhood.consider_swap(&mut person_indices, layer, first, first + 1);
                if neighbourhood.exhausted() {
                    break;
                }
            }
            if neighbourhood.exhausted() {
                break;
            }
        }

        // Empty columns can separate otherwise congested connection bundles.
        if !neighbourhood.exhausted() {
            for layer in 0..person_indices.len() {
                let occupied = person_indices[layer]
                    .iter()
                    .map(|person| person.index)
                    .collect::<Vec<_>>();
                for person in 0..person_indices[layer].len() {
                    for column in 0..row_length {
                        if !occupied.contains(&column) {
                            neighbourhood.consider_move(&mut person_indices, layer, person, column);
                        }
                        if neighbourhood.exhausted() {
                            break;
                        }
                    }
                    if neighbourhood.exhausted() {
                        break;
                    }
                }
                if neighbourhood.exhausted() {
                    break;
                }
            }
        }

        // Non-adjacent swaps allow a bounded escape from purely local orderings.
        if !neighbourhood.exhausted() {
            for layer in 0..person_indices.len() {
                for first in 0..person_indices[layer].len() {
                    for second in first + 2..person_indices[layer].len() {
                        neighbourhood.consider_swap(&mut person_indices, layer, first, second);
                        if neighbourhood.exhausted() {
                            break;
                        }
                    }
                    if neighbourhood.exhausted() {
                        break;
                    }
                }
                if neighbourhood.exhausted() {
                    break;
                }
            }
        }

        let Some(best_move) = neighbourhood.best_move else {
            break;
        };
        apply_move(&mut person_indices, best_move.kind);
        current_score = best_move.score;
        if evaluations >= MAX_EVALUATIONS_PER_WIDTH {
            break;
        }
    }

    Candidate {
        person_indices,
        row_length,
        score: current_score,
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
    evaluations: &'a mut usize,
}

impl Neighbourhood<'_, '_> {
    fn exhausted(&self) -> bool {
        *self.evaluations >= MAX_EVALUATIONS_PER_WIDTH
    }

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
        *self.evaluations += 1;
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

    for channel in rel_indices
        .iter()
        .flat_map(|row| lines::create_horizontal(row))
    {
        let mut occupation = vec![0_u64; row_length];
        for line in channel {
            length = length.saturating_add(line.end.abs_diff(line.start) as u64);
            for count in &mut occupation[line.start..=line.end] {
                congestion = congestion.saturating_add(*count);
                *count += 1;
            }
        }
    }

    Score { congestion, length }
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
}
