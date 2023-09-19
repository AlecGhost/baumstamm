use super::Grid;
use baumstamm_lib::Relationship;
use itertools::Itertools;

type Pid = baumstamm_lib::PersonId;
type Rid = baumstamm_lib::RelationshipId;

#[derive(Clone, Debug, Default)]
pub struct RelIndices {
    pub parents: [Option<usize>; 2],
    pub children: Vec<usize>,
    pub crossing_point: Option<usize>,
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
    let mut rel_indices = layers
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
                    let mut rel_indices = RelIndices::default();
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
        .collect_vec();
    rel_indices.push(Vec::new());
    rel_indices
}

pub fn get_person_indices(
    person_layers: &Grid<Pid>,
    row_length: usize,
    rels: &[Relationship],
) -> Grid<PersonIndex> {
    enum Dir {
        Children,
        Parents,
    }

    fn sort_to_ref(
        pids: &[Pid],
        _ref_row: &[PersonIndex],
        _dir: Dir,
        _rels: &[Relationship],
    ) -> Vec<PersonIndex> {
        pids.iter()
            .enumerate()
            .map(|(index, pid)| PersonIndex { index, pid: *pid })
            .collect_vec()
    }

    let mut person_indices: Grid<PersonIndex> = Vec::new();
    let (ref_index, ref_layer) = person_layers
        .iter()
        .find_position(|layer| layer.len() == row_length)
        .expect("There must be a longest row");
    let ref_row = ref_layer
        .iter()
        .enumerate()
        .map(|(index, pid)| PersonIndex { index, pid: *pid })
        .collect_vec();
    let mut upper_rows = person_layers[..ref_index]
        .iter()
        .rev()
        .fold(
            (ref_row.clone(), Vec::new()),
            |(ref_row, mut rows), layer| {
                let row = sort_to_ref(layer, &ref_row, Dir::Parents, rels);
                rows.push(row.clone());
                (row, rows)
            },
        )
        .1;
    upper_rows.reverse();
    let lower_rows = person_layers[(ref_index + 1)..]
        .iter()
        .fold(
            (ref_row.clone(), Vec::new()),
            |(ref_row, mut rows), layer| {
                let row = sort_to_ref(layer, &ref_row, Dir::Children, rels);
                rows.push(row.clone());
                (row, rows)
            },
        )
        .1;
    person_indices.extend(upper_rows);
    person_indices.push(ref_row);
    person_indices.extend(lower_rows);
    person_indices
}

fn middle(a: usize, b: usize) -> usize {
    let diff = if a <= b { b - a } else { a - b };
    if diff % 2 == 0 {
        diff / 2
    } else {
        (diff - 1) / 2
    }
}
