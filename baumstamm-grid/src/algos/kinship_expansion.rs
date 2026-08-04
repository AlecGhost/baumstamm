use super::{PersonIndexInput, PersonIndexOutput};
use crate::indices::PersonIndex;
use baumstamm_lib::{PersonId, Relationship};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

type Pid = PersonId;

pub(super) fn get_person_indices(input: PersonIndexInput<'_>) -> PersonIndexOutput {
    let occurrence_order = input
        .person_layers
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let first_occurrence = occurrence_order.iter().copied().enumerate().fold(
        HashMap::new(),
        |mut positions, (position, pid)| {
            positions.entry(pid).or_insert(position);
            positions
        },
    );

    let Some(&root) = occurrence_order.first() else {
        return PersonIndexOutput {
            person_indices: Vec::new(),
            row_length: 0,
        };
    };

    let neighbours = immediate_family(input.relationships, &first_occurrence);
    let expansion = breadth_first_people(root, &occurrence_order, &neighbours, &first_occurrence);
    let coordinates = expansion_coordinates(&expansion);
    let minimum = coordinates.values().copied().min().unwrap_or_default();
    let maximum = coordinates.values().copied().max().unwrap_or_default();
    let row_length = usize::try_from(maximum - minimum + 1).expect("layout width must fit usize");

    let person_indices = input
        .person_layers
        .iter()
        .map(|layer| {
            let indices = layer
                .iter()
                .map(|pid| PersonIndex {
                    pid: *pid,
                    index: usize::try_from(coordinates[pid] - minimum)
                        .expect("normalized coordinate must be non-negative"),
                })
                .collect::<Vec<_>>();
            debug_assert_eq!(
                indices
                    .iter()
                    .map(|person| person.index)
                    .collect::<HashSet<_>>()
                    .len(),
                indices.len(),
                "people in one layer must occupy distinct columns"
            );
            indices
        })
        .collect();

    PersonIndexOutput {
        person_indices,
        row_length,
    }
}

// Treat everyone named by one relationship as immediate family. This makes
// spouses, parents, children, and siblings one expansion step apart while
// keeping the traversal independent of the cut graph's repeated occurrences.
fn immediate_family(
    relationships: &[Relationship],
    visible_people: &HashMap<Pid, usize>,
) -> BTreeMap<Pid, BTreeSet<Pid>> {
    let mut neighbours = BTreeMap::<Pid, BTreeSet<Pid>>::new();
    for relationship in relationships {
        let members = relationship
            .parents
            .iter()
            .flatten()
            .chain(&relationship.children)
            .copied()
            .filter(|pid| visible_people.contains_key(pid))
            .collect::<Vec<_>>();
        for (index, person) in members.iter().enumerate() {
            for relative in members.iter().skip(index + 1) {
                if person != relative {
                    neighbours.entry(*person).or_default().insert(*relative);
                    neighbours.entry(*relative).or_default().insert(*person);
                }
            }
        }
    }
    neighbours
}

fn breadth_first_people(
    root: Pid,
    occurrence_order: &[Pid],
    neighbours: &BTreeMap<Pid, BTreeSet<Pid>>,
    first_occurrence: &HashMap<Pid, usize>,
) -> Vec<(Pid, usize)> {
    let mut expansion = Vec::new();
    let mut visited = BTreeSet::new();
    let mut maximum_distance = 0;

    for component_root in std::iter::once(root).chain(occurrence_order.iter().copied()) {
        if !visited.insert(component_root) {
            continue;
        }
        let component_distance = if expansion.is_empty() {
            0
        } else {
            maximum_distance + 1
        };
        let mut queue = VecDeque::from([(component_root, component_distance)]);
        while let Some((person, distance)) = queue.pop_front() {
            maximum_distance = maximum_distance.max(distance);
            expansion.push((person, distance));
            let mut relatives = neighbours
                .get(&person)
                .into_iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            relatives.sort_by_key(|relative| (first_occurrence[relative], *relative));
            for relative in relatives {
                if visited.insert(relative) {
                    queue.push_back((relative, distance + 1));
                }
            }
        }
    }
    expansion
}

fn expansion_coordinates(expansion: &[(Pid, usize)]) -> BTreeMap<Pid, isize> {
    let mut coordinates = BTreeMap::new();
    let mut start = 0;
    let mut next_radius = 0_isize;
    let mut start_on_left = true;
    while start < expansion.len() {
        let distance = expansion[start].1;
        let end = expansion[start..]
            .iter()
            .position(|(_, candidate_distance)| *candidate_distance != distance)
            .map_or(expansion.len(), |offset| start + offset);
        if distance == 0 {
            coordinates.insert(expansion[start].0, 0);
        } else {
            next_radius += 1;
            for (offset, (pid, _)) in expansion[start..end].iter().enumerate() {
                let radius =
                    next_radius + isize::try_from(offset / 2).expect("layout width must fit isize");
                let left = (offset % 2 == 0) == start_on_left;
                coordinates.insert(*pid, if left { -radius } else { radius });
            }
            next_radius +=
                isize::try_from((end - start - 1) / 2).expect("layout width must fit isize");
            start_on_left = !start_on_left;
        }
        start = end;
    }
    coordinates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algos::LayoutAlgorithm;
    use baumstamm_lib::RelationshipId;

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
        let relationship_layers = Vec::new();
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
    fn expands_by_kinship_distance_from_the_oldest_layer_root() {
        let layers = vec![vec![pid(1), pid(9)], vec![pid(2)], vec![pid(3), pid(4)]];
        let relationships = vec![relationship(10, &[1, 2]), relationship(11, &[2, 3])];
        let output = run(&layers, &relationships);
        let root_column = column(&output, pid(1));

        assert_eq!(output.row_length, 8);
        assert_eq!(column(&output, pid(2)).abs_diff(root_column), 1);
        assert!(column(&output, pid(3)).abs_diff(root_column) > 1);
        assert!(column(&output, pid(9)).abs_diff(root_column) > 1);
    }

    #[test]
    fn repeated_people_stay_on_every_layer_and_align_without_collisions() {
        let layers = vec![vec![pid(1), pid(2)], vec![pid(2), pid(3)], vec![pid(4)]];
        let relationships = vec![relationship(10, &[1, 2, 3]), relationship(11, &[3, 4])];
        let first = run(&layers, &relationships);
        let second = run(&layers, &relationships);

        assert_eq!(column(&first, pid(2)), first.person_indices[1][0].index);
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
}
