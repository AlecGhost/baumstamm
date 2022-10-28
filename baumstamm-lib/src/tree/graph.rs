use super::{PersonId, Relationship};
use crate::util::UniqueIterator;

pub(super) fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

pub(super) fn child_relationship<'a>(
    id: &PersonId,
    relationships: &'a [Relationship],
) -> &'a Relationship {
    relationships
        .iter()
        .filter(|rel| rel.children.contains(id))
        .collect::<Vec<&Relationship>>()
        .first()
        // TODO: change signature to result
        .expect("Inconsistent data")
}

pub(super) fn parent_relationships<'a>(
    id: &PersonId,
    relationships: &'a [Relationship],
) -> Vec<&'a Relationship> {
    relationships
        .iter()
        .filter(|rel| rel.parents().contains(id))
        .collect()
}

// pub(super) fn generation_matrix(relationships: &[Relationship]) -> Vec<Vec<PersonId>> {
//     fn fill_matrix(
//         matrix: &mut Vec<Vec<PersonId>>,
//         rel: &Relationship,
//         relationships: &[Relationship],
//         level: usize,
//     ) {
//         // println!("rel: {}, level: {}", rel.id, level);
//         // println!("{:?}", matrix);
//         if matrix.len() == level {
//             let new_row: Vec<PersonId> = Vec::new();
//             matrix.push(new_row);
//         }
//         let row = matrix.get_mut(level).expect("Invalid matrix level");
//         if rel.children.iter().any(|child| row.contains(child)) {
//             return;
//         }
//         rel.children
//             .iter()
//             .for_each(|child| row.push(child.clone()));
//         // if level != 0 {
//         //     rel.parents().iter().for_each(|parent| {
//         //         let child_rel = child_relationship(parent, relationships);
//         //         fill_matrix(matrix, child_rel, relationships, level - 1)
//         //     });
//         // }
//         rel.children.iter().for_each(|child| {
//             let parent_rel = parent_relationships(child, relationships);
//             parent_rel.iter().for_each(|rel| {
//                 fill_matrix(matrix, rel, relationships, level + 1);
//             });
//         });
//     }

//     let mut matrix = Vec::new();
//     let oldest = relationships
//         .iter()
//         .filter(|rel| rel.parents().is_empty())
//         // .reduce(|accum, rel| {
//         //     if accum.generations_below(relationships) > rel.generations_below(relationships) {
//         //         accum
//         //     } else {
//         //         rel
//         //     }
//         // })
//         // .expect("There must be an oldest generation");
//         .for_each(|start| {
//             fill_matrix(&mut matrix, start, relationships, 0);
//         });
//     matrix
// }

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::tree::io;

//     #[test]
//     fn test_generation_matrix() {
//         let relationships =
//             io::read_relationships("test/generation_matrix.json").expect("Cannot read test file");
//         println!(
//             "Generation matrix:\n{:?}",
//             generation_matrix(&relationships)
//         );
//         panic!("Test successful");
//     }
// }
