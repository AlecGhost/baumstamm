use crate::grid::PersonInfo;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
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
}

fn read_relationships(file_name: &str) -> Result<Vec<Relationship>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let json: Vec<Relationship> = serde_json::from_str(&json_str)?;
    return Ok(json);
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
    return Ok(json);
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
}
