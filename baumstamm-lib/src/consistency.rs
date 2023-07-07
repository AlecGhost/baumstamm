use crate::{graph, Person, PersonId, Relationship, TreeData};
use itertools::Itertools;
use std::{collections::HashMap, iter::FromIterator};

pub(super) fn check(tree_data: &TreeData) -> Result<(), &'static str> {
    check_relationships(&tree_data.relationships)?;
    check_persons(&tree_data.persons)?;

    // turn into hash map for O(n) access
    let persons_hashmap: HashMap<PersonId, ()> = HashMap::from_iter(
        graph::extract_persons(&tree_data.relationships)
            .iter()
            .map(|person_id| (*person_id, ())),
    );
    if tree_data.persons.len() != persons_hashmap.len() {
        return Err("The number of persons differs");
    }
    if tree_data
        .persons
        .iter()
        .map(|person| person.id)
        .any(|person_id| !persons_hashmap.contains_key(&person_id))
    {
        return Err("Relationships and persons do not match");
    }

    Ok(())
}

fn check_relationships(relationships: &Vec<Relationship>) -> Result<(), &'static str> {
    if relationships.is_empty() {
        return Ok(());
    }

    if relationships.len() != relationships.iter().map(|rel| rel.id).unique().count() {
        return Err("More than one relationship with the same id");
    }

    if relationships
        .iter()
        .filter(|rel| !rel.parents().is_empty())
        .any(|rel| rel.parents[0] == rel.parents[1])
    {
        return Err("Self referencing relationship");
    }

    if relationships.iter().any(|rel| {
        rel.children
            .iter()
            .any(|child| rel.parents().iter().any(|parent| parent == child))
    }) {
        return Err("A Child cannot be its parent");
    }

    // Relationship.descendants() is safe to call after this check
    let children = relationships
        .iter()
        .flat_map(|rel| rel.children.clone())
        .collect::<Vec<PersonId>>();

    if children.len() != graph::extract_persons(relationships).len() {
        return Err("Every person must be child of a relationship");
    }

    if children.len() != children.iter().unique().count() {
        return Err("A Person is child of more than one relationship");
    }

    fn nr_connected_persons(relationships: &[Relationship]) -> usize {
        let mut total_related_persons = relationships[0].persons();
        let mut index = 0;

        while index < total_related_persons.len() {
            let current_person = total_related_persons[index];
            relationships
                .iter()
                .filter(|rel| rel.persons().contains(&current_person))
                .flat_map(|rel| rel.persons())
                .unique()
                .for_each(|person| {
                    if !total_related_persons.contains(&person) {
                        total_related_persons.push(person);
                    }
                });
            index += 1;
        }

        total_related_persons.len()
    }

    let nr_persons = children.len();
    if nr_connected_persons(relationships) != nr_persons {
        return Err("Not all nodes are connected");
    }

    if relationships.iter().any(|rel| {
        rel.parents()
            .iter()
            .any(|parent| rel.descendants(relationships).contains(parent))
    }) {
        return Err("Cycle in family tree");
    }

    Ok(())
}

fn check_persons(persons: &[Person]) -> Result<(), &'static str> {
    let person_ids: Vec<PersonId> = persons.iter().map(|person| person.id).collect();

    if person_ids.len() != person_ids.iter().unique().count() {
        return Err("Multiple persons with the same id");
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::io;

    enum FileType {
        Relationships,
        Persons,
    }

    fn assert_err_for_file(expected_err_message: &str, file_name: &str, file_type: FileType) {
        let tree_data = io::read(file_name).expect("Cannot read test file");
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
    fn check_both() -> Result<(), &'static str> {
        let tree_data =
            io::read("test/consistency/check_both.json").expect("Cannot read test file");
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
        let tree_data =
            io::read("test/consistency/too_few_persons.json").expect("Cannot read test file");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("The number of persons differs", format!("{err}"));
    }

    #[test]
    fn too_few_rels() {
        let tree_data =
            io::read("test/consistency/too_few_rels.json").expect("Cannot read test file");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("The number of persons differs", format!("{err}"));
    }

    #[test]
    fn different_ids() {
        let tree_data =
            io::read("test/consistency/different_ids.json").expect("Cannot read test file");
        let err = check(&tree_data).expect_err("Consistency check failed");
        assert_eq!("Relationships and persons do not match", format!("{err}"));
    }
}
