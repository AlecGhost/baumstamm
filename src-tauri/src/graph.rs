use crate::grid::PersonInfo;
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
}

fn read_relationships(file_name: &str) -> Result<Vec<Relationship>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let json: Vec<Relationship> = serde_json::from_str(&json_str)?;
    relationships_consistancy_check(&json)?;
    Ok(json)
}

fn relationships_consistancy_check(relationships: &Vec<Relationship>) -> Result<(), String> {
    if relationships.len() != relationships.iter().unique().count() {
        return Err("More than one relationship with the same id".to_string());
    }
    if relationships
        .iter()
        .filter(|rel| rel.p1 != None && rel.p2 != None)
        .any(|rel| rel.p1 == rel.p2)
    {
        return Err("Self referencing Relationship".to_string());
    }

    Ok(())
}

fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| {
        vec![rel.p1, rel.p2]
            .iter()
            .flatten()
            .cloned()
            .collect::<Vec<PersonId>>()
    });
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

struct Unique<T, U> {
    iter: T,
    evaluated: Vec<U>,
}

impl<I, T> Iterator for Unique<I, T>
where
    I: Iterator<Item = T>,
    T: std::cmp::PartialEq + Clone,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let next_item = self.iter.next()?;
        if self.evaluated.contains(&next_item) {
            self.next()
        } else {
            self.evaluated = self
                .evaluated
                .iter()
                .cloned()
                .chain(vec![next_item.clone()])
                .collect();
            Some(next_item)
        }
    }
}

trait UniqueIterator<T>: Iterator<Item = T> + Sized {
    fn unique(self) -> Unique<Self, T> {
        Unique {
            iter: self,
            evaluated: Vec::new(),
        }
    }
}

impl<T, I: Iterator<Item = T>> UniqueIterator<T> for I {}

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
}
