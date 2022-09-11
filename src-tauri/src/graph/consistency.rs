use crate::util::UniqueIterator;

use super::{Person, PersonId, Relationship};

pub fn check_relationships(relationships: &Vec<Relationship>) -> Result<(), &'static str> {
    if relationships.is_empty() {
        return Ok(());
    }

    if relationships.len() != relationships.iter().map(|rel| rel.id).unique().count() {
        return Err("More than one relationship with the same id");
    }

    if relationships
        .iter()
        .filter(|rel| rel.p1 != None && rel.p2 != None)
        .any(|rel| rel.p1 == rel.p2)
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

    if children.len() != super::extract_persons(relationships).len() {
        return Err("Every person must be child of a relationship");
    }

    if children.len() != children.iter().unique().count() {
        return Err("A Person is child of more than one relationship");
    }

    fn nr_connected_persons(relationships: &[Relationship]) -> usize {
        let mut total_related_persons = relationships[0].persons();
        let mut index = 0;

        while index < total_related_persons.len() {
            let current_person = &total_related_persons[index].clone();
            relationships
                .iter()
                .filter(|rel| rel.persons().contains(current_person))
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

pub fn check_persons(persons: &[Person]) -> Result<(), &'static str> {
    let person_ids: Vec<PersonId> = persons.iter().map(|person| person.id).collect();

    if person_ids.len() != person_ids.iter().unique().count() {
        return Err("Multiple persons with the same id");
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::graph::io::{read_persons, read_relationships};
    use std::error::Error;

    fn test_err_message(expected_err_message: &str, err: Box<dyn Error>) {
        assert_eq!(expected_err_message, format!("{err}"))
    }

    #[test]
    fn multiple_ids() {
        test_err_message(
            "More than one relationship with the same id",
            read_relationships("test/multiple_ids.json").expect_err("Multiple ids failed"),
        );
    }

    #[test]
    fn self_reference() {
        test_err_message(
            "Self referencing relationship",
            read_relationships("test/self_reference.json").expect_err("Self reference failed"),
        );
    }

    #[test]
    fn child_is_parent() {
        test_err_message(
            "A Child cannot be its parent",
            read_relationships("test/child_is_parent.json").expect_err("Child is parent failed"),
        );
    }

    #[test]
    fn child_of_relationship() {
        test_err_message(
            "Every person must be child of a relationship",
            read_relationships("test/child_of_relationship.json")
                .expect_err("Child of relationship failed"),
        );
    }

    #[test]
    fn more_than_one_parent_rel() {
        test_err_message(
            "A Person is child of more than one relationship",
            read_relationships("test/more_than_one_parent_rel.json")
                .expect_err("More than one parent failed"),
        );
    }

    #[test]
    fn everything_connected() {
        test_err_message(
            "Not all nodes are connected",
            read_relationships("test/everything_connected.json")
                .expect_err("Everything connected failed"),
        );
    }

    #[test]
    fn no_cycles() {
        test_err_message(
            "Cycle in family tree",
            read_relationships("test/no_cycles.json").expect_err("No cycle failed"),
        );
    }

    #[test]
    fn person_multiple_ids() {
        test_err_message(
            "Multiple persons with the same id",
            read_persons("test/person_multiple_ids.json").expect_err("Person multiple ids failed"),
        );
    }
}
