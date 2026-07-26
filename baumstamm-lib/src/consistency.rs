use crate::{error::ConsistencyError, Person, PersonId, Relationship, TreeData};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn check(tree_data: &TreeData) -> Result<(), ConsistencyError> {
    let referenced_persons = check_relationships(&tree_data.relationships)?;
    check_persons(&tree_data.persons)?;

    if tree_data.persons.len() != referenced_persons.len() {
        return Err(ConsistencyError::DifferentNumberOfPersons);
    }
    if tree_data
        .persons
        .iter()
        .map(|person| person.id)
        .any(|person_id| !referenced_persons.contains(&person_id))
    {
        return Err(ConsistencyError::UnmatchedQuantity);
    }

    Ok(())
}

struct RelationshipIndex {
    referenced_persons: HashSet<PersonId>,
    child_count: usize,
    children_are_unique: bool,
    relationships_by_person: HashMap<PersonId, Vec<usize>>,
    children_by_parent: HashMap<PersonId, Vec<PersonId>>,
}

impl RelationshipIndex {
    fn new(relationships: &[Relationship]) -> Self {
        let person_reference_count = relationships
            .iter()
            .map(|relationship| {
                relationship.children.len() + relationship.parents.iter().flatten().count()
            })
            .sum();
        let mut referenced_persons = HashSet::with_capacity(person_reference_count);
        let mut unique_children = HashSet::with_capacity(person_reference_count);
        let mut child_count = 0;
        let mut children_are_unique = true;
        let mut relationships_by_person =
            HashMap::<PersonId, Vec<usize>>::with_capacity(person_reference_count);
        let mut children_by_parent = HashMap::<PersonId, Vec<PersonId>>::new();

        for (relationship_index, relationship) in relationships.iter().enumerate() {
            for person in relationship
                .parents
                .iter()
                .flatten()
                .chain(&relationship.children)
            {
                referenced_persons.insert(*person);
                relationships_by_person
                    .entry(*person)
                    .or_default()
                    .push(relationship_index);
            }

            child_count += relationship.children.len();
            children_are_unique &= relationship
                .children
                .iter()
                .all(|child| unique_children.insert(*child));
            for parent in relationship.parents.iter().flatten() {
                children_by_parent
                    .entry(*parent)
                    .or_default()
                    .extend(relationship.children.iter().copied());
            }
        }

        Self {
            referenced_persons,
            child_count,
            children_are_unique,
            relationships_by_person,
            children_by_parent,
        }
    }

    fn nr_connected_persons(&self, relationships: &[Relationship]) -> usize {
        let Some(first_person) = relationships.first().and_then(|relationship| {
            relationship
                .parents
                .iter()
                .flatten()
                .chain(&relationship.children)
                .next()
                .copied()
        }) else {
            return 0;
        };

        let mut visited_persons = HashSet::with_capacity(self.referenced_persons.len());
        let mut visited_relationships = vec![false; relationships.len()];
        let mut pending_persons = VecDeque::new();
        visited_persons.insert(first_person);
        pending_persons.push_back(first_person);

        while let Some(person) = pending_persons.pop_front() {
            let Some(related_relationships) = self.relationships_by_person.get(&person) else {
                continue;
            };
            for &relationship_index in related_relationships {
                if visited_relationships[relationship_index] {
                    continue;
                }
                visited_relationships[relationship_index] = true;

                let relationship = &relationships[relationship_index];
                for related_person in relationship
                    .parents
                    .iter()
                    .flatten()
                    .chain(&relationship.children)
                {
                    if visited_persons.insert(*related_person) {
                        pending_persons.push_back(*related_person);
                    }
                }
            }
        }

        visited_persons.len()
    }

    fn has_cycle(&self) -> bool {
        let mut indegrees = self
            .referenced_persons
            .iter()
            .map(|person| (*person, 0_usize))
            .collect::<HashMap<_, _>>();
        for children in self.children_by_parent.values() {
            for child in children {
                *indegrees
                    .get_mut(child)
                    .expect("children are indexed as referenced persons") += 1;
            }
        }

        let mut pending_persons = indegrees
            .iter()
            .filter_map(|(person, indegree)| (*indegree == 0).then_some(*person))
            .collect::<VecDeque<_>>();
        let mut visited = 0;

        while let Some(person) = pending_persons.pop_front() {
            visited += 1;
            let Some(children) = self.children_by_parent.get(&person) else {
                continue;
            };
            for child in children {
                let indegree = indegrees
                    .get_mut(child)
                    .expect("children are indexed as referenced persons");
                *indegree -= 1;
                if *indegree == 0 {
                    pending_persons.push_back(*child);
                }
            }
        }

        visited != self.referenced_persons.len()
    }
}

fn check_relationships(
    relationships: &[Relationship],
) -> Result<HashSet<PersonId>, ConsistencyError> {
    if relationships.is_empty() {
        return Ok(HashSet::new());
    }

    let mut relationship_ids = HashSet::with_capacity(relationships.len());
    if relationships
        .iter()
        .any(|relationship| !relationship_ids.insert(relationship.id))
    {
        return Err(ConsistencyError::RelationshipIdExists);
    }

    let mut parent_pairs = HashSet::with_capacity(relationships.len());
    if relationships.iter().any(|relationship| {
        let [Some(parent1), Some(parent2)] = relationship.parents else {
            return false;
        };
        let pair = if parent1 <= parent2 {
            [parent1, parent2]
        } else {
            [parent2, parent1]
        };
        !parent_pairs.insert(pair)
    }) {
        return Err(ConsistencyError::RelationshipExists);
    }

    if relationships
        .iter()
        .any(|relationship| matches!(relationship.parents, [Some(p1), Some(p2)] if p1 == p2))
    {
        return Err(ConsistencyError::SelfReference);
    }

    if relationships.iter().any(|relationship| {
        relationship.children.iter().any(|child| {
            relationship
                .parents
                .iter()
                .flatten()
                .any(|parent| parent == child)
        })
    }) {
        return Err(ConsistencyError::DirectCycle);
    }

    let index = RelationshipIndex::new(relationships);

    if index.child_count != index.referenced_persons.len() {
        return Err(ConsistencyError::MustBeChild);
    }

    if !index.children_are_unique {
        return Err(ConsistencyError::MoreThanOnceChild);
    }

    if index.nr_connected_persons(relationships) != index.child_count {
        return Err(ConsistencyError::Unconnected);
    }

    if index.has_cycle() {
        return Err(ConsistencyError::IndirectCycle);
    }

    Ok(index.referenced_persons)
}

fn check_persons(persons: &[Person]) -> Result<(), ConsistencyError> {
    let mut person_ids = HashSet::with_capacity(persons.len());
    if persons.iter().any(|person| !person_ids.insert(person.id)) {
        return Err(ConsistencyError::PersonIdExists);
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    fn read(file_name: &str) -> TreeData {
        let json_data = std::fs::read_to_string(file_name).expect("Cannot read test file");
        crate::io::read(&json_data).expect("Cannot convert test file")
    }

    enum FileType {
        Relationships,
        Persons,
    }

    fn assert_err_for_file(expected_err_message: &str, file_name: &str, file_type: FileType) {
        let tree_data = read(file_name);
        let err = match file_type {
            FileType::Relationships => {
                check_relationships(&tree_data.relationships).expect_err("Consistency check failed")
            }
            FileType::Persons => {
                check_persons(&tree_data.persons).expect_err("Consistency check failed")
            }
        };
        assert_eq!(expected_err_message, format!("{err}"))
    }

    #[test]
    fn multiple_ids() {
        assert_err_for_file(
            "More than one relationship with the same id",
            "test/consistency/multiple_ids.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn self_reference() {
        assert_err_for_file(
            "Self referencing relationship",
            "test/consistency/self_reference.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn child_is_parent() {
        assert_err_for_file(
            "A Child cannot be its parent",
            "test/consistency/child_is_parent.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn child_of_relationship() {
        assert_err_for_file(
            "Every person must be child of a relationship",
            "test/consistency/child_of_relationship.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn more_than_one_parent_rel() {
        assert_err_for_file(
            "A Person is child of more than one relationship",
            "test/consistency/more_than_one_parent_rel.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn everything_connected() {
        assert_err_for_file(
            "Not all nodes are connected",
            "test/consistency/everything_connected.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn no_cycles() {
        assert_err_for_file(
            "Cycle in family tree",
            "test/consistency/no_cycles.json",
            FileType::Relationships,
        );
    }

    #[test]
    fn person_multiple_ids() {
        assert_err_for_file(
            "Multiple persons with the same id",
            "test/consistency/person_multiple_ids.json",
            FileType::Persons,
        );
    }

    #[test]
    fn check_both() -> Result<(), ConsistencyError> {
        let tree_data = read("test/consistency/check_both.json");
        match check(&tree_data) {
            Ok(_) => Ok(()),
            Err(err) => {
                println!("{err}");
                Err(err)
            }
        }
    }

    #[test]
    fn too_few_persons() {
        let tree_data = read("test/consistency/too_few_persons.json");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("The number of persons differs", format!("{err}"));
    }

    #[test]
    fn too_few_rels() {
        let tree_data = read("test/consistency/too_few_rels.json");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("The number of persons differs", format!("{err}"));
    }

    #[test]
    fn different_ids() {
        let tree_data = read("test/consistency/different_ids.json");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("Relationships and persons do not match", format!("{err}"));
    }
}
