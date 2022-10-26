use super::{PersonId, Relationship};
use crate::util::UniqueIterator;

pub(super) fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

pub(super) fn child_relationship(id: PersonId, relationships: &[Relationship]) -> &Relationship {
    relationships
        .iter()
        .filter(|rel| rel.children.contains(&id))
        .collect::<Vec<&Relationship>>()
        .first()
        .expect("Inconsistent data")
}

pub(super) fn parent_relationships(
    id: PersonId,
    relationships: &[Relationship],
) -> Vec<&Relationship> {
    relationships
        .iter()
        .filter(|rel| rel.parents().contains(&id))
        .collect()
}

// pub(super) fn generations(relationships: &[Relationship]) -> HashMap<RelationshipId, Vec<u32>> {
//     fn add(map: &mut HashMap<RelationshipId, Vec<u32>>, rel_id: RelationshipId, gen: u32) {
//         let gens = map.entry(rel_id).or_insert(Vec::new());
//         if !gens.contains(&gen) {
//             gens.push(gen)
//         }
//     }
//     let mut map = HashMap::new();
//     let oldest = relationships
//         .iter()
//         .filter(|rel| rel.parents().len() == 0)
//         .reduce(|accum, rel| {
//             if accum.generations_below(relationships) > rel.generations_below(relationships) {
//                 accum
//             } else {
//                 rel
//             }
//         })
//         .expect("There must be an oldest generation");
//     add(&mut map, oldest.id, 0);
//     map
// }
