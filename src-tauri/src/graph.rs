use crate::grid::PersonInfo;
use crate::util::UniqueIterator;
#[allow(unused_imports)]
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
#[allow(unused_imports)]
use uuid::{uuid, Uuid};

type PersonId = Uuid;
type RelationshipId = Uuid;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Relationship {
    id: RelationshipId,
    p1: Option<PersonId>,
    p2: Option<PersonId>,
    children: Vec<PersonId>,
}

impl Relationship {
    fn new(p1: Option<PersonId>, p2: Option<PersonId>, children: Vec<PersonId>) -> Self {
        Self {
            id: PersonId::new_v4(),
            p1,
            p2,
            children,
        }
    }

    fn parents(&self) -> Vec<PersonId> {
        vec![self.p1, self.p2].iter().flatten().cloned().collect()
    }

    fn persons(&self) -> Vec<PersonId> {
        self.parents()
            .iter()
            .cloned()
            .chain(self.children.clone())
            .collect()
    }

    fn descendants(&self, relationships: &[Relationship]) -> Vec<PersonId> {
        let mut descendants = self.children.clone();
        let mut index = 0;
        while index < descendants.len() {
            let descendant = &descendants[index].clone();
            relationships
                .iter()
                .filter(|rel| rel.parents().contains(descendant))
                .flat_map(|rel| rel.children.clone())
                .unique()
                .for_each(|child| {
                    if !descendants.contains(&child) {
                        descendants.push(child)
                    }
                });

            index += 1;
        }
        descendants
    }
}

fn read_relationships(file_name: &str) -> Result<Vec<Relationship>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let json: Vec<Relationship> = serde_json::from_str(&json_str)?;
    relationships_consistancy_check(&json)?;
    Ok(json)
}

fn relationships_consistancy_check(relationships: &Vec<Relationship>) -> Result<(), String> {
    if relationships.is_empty() {
        return Ok(());
    }

    if relationships.len() != relationships.iter().map(|rel| rel.id).unique().count() {
        return Err("More than one relationship with the same id".to_string());
    }

    if relationships
        .iter()
        .filter(|rel| rel.p1 != None && rel.p2 != None)
        .any(|rel| rel.p1 == rel.p2)
    {
        return Err("Self referencing relationship".to_string());
    }

    if relationships.iter().any(|rel| {
        rel.children
            .iter()
            .any(|child| rel.parents().iter().any(|parent| parent == child))
    }) {
        return Err("A Child cannot be its parent".to_string());
    }

    // Relationship.descendants() is safe to call after this check
    let children = relationships
        .iter()
        .flat_map(|rel| rel.children.clone())
        .collect::<Vec<PersonId>>();

    if children.len() != extract_persons(relationships).len() {
        return Err("Every person must be child of a relationship".to_string());
    }

    if children.len() != children.iter().unique().count() {
        return Err("A Person is child of more than one relationship".to_string());
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
        return Err("Not all nodes are connected".to_string());
    }

    if relationships.iter().any(|rel| {
        rel.parents()
            .iter()
            .any(|parent| rel.descendants(relationships).contains(parent))
    }) {
        return Err("Cycle in family tree".to_string());
    }

    Ok(())
}

fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Person {
    id: PersonId,
    info: Option<PersonInfo>,
}

impl Person {
    fn new() -> Self {
        Person {
            id: PersonId::new_v4(),
            info: None,
        }
    }

    fn add_info(self, info: PersonInfo) -> Self {
        Self {
            info: Some(info),
            ..self
        }
    }
}

fn read_persons(file_name: &str) -> Result<Vec<Person>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let json: Vec<Person> = serde_json::from_str(&json_str)?;
    Ok(json)
}

#[cfg(test)]
mod test {
    use super::*;

    fn compare_rels_to_file(
        test_rels: Vec<Relationship>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let rels = match read_relationships(file_name) {
            Ok(value) => value,
            Err(_) => return Err("Deserialization failed".into()),
        };
        if rels == test_rels {
            Ok(())
        } else {
            let message = format!(
                "Json does not match\nInput: {:?}\nExpected: {:?}",
                rels, test_rels
            );
            Err(message.into())
        }
    }

    #[test]
    fn empty_rels() -> Result<(), Box<dyn Error>> {
        compare_rels_to_file(Vec::new(), "test/empty_rels.json")
    }

    #[test]
    fn single_rel() -> Result<(), Box<dyn Error>> {
        let test_rels = vec![Relationship {
            id: uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            p1: None,
            p2: None,
            children: Vec::new(),
        }];
        compare_rels_to_file(test_rels, "test/single_rel.json")
    }

    fn compare_persons_to_file(
        test_persons: Vec<Person>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let persons = match read_persons(file_name) {
            Ok(value) => value,
            Err(_) => return Err("Deserialization failed".into()),
        };
        if persons == test_persons {
            Ok(())
        } else {
            let message = format!(
                "Json does not match\nInput: {:?}\nExpected: {:?}",
                persons, test_persons
            );
            Err(message.into())
        }
    }

    #[test]
    fn empty_persons() -> Result<(), Box<dyn Error>> {
        compare_persons_to_file(Vec::new(), "test/empty_persons.json")
    }

    #[test]
    fn single_person() -> Result<(), Box<dyn Error>> {
        let test_persons = vec![Person {
            id: uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            info: None,
        }];
        compare_persons_to_file(test_persons, "test/single_person.json")
    }

    #[test]
    fn unique_persons() -> Result<(), Box<dyn Error>> {
        let relationships = read_relationships("test/unique_persons.json")?;
        let unique_persons = extract_persons(&relationships);
        let test_persons = vec![
            uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            uuid!("57e55044-10b1-426f-9247-bb680e5fe0c8"),
        ];
        if unique_persons == test_persons {
            Ok(())
        } else {
            let message = format!(
                "Unique Extraction failed
Input: {:?}
Expected: {:?}",
                unique_persons, test_persons
            );
            Err(message.into())
        }
    }

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
}
