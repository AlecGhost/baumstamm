use std::{error::Error, fs};

use super::{Person, Relationship};

pub(super) fn read_relationships(file_name: &str) -> Result<Vec<Relationship>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let relationships: Vec<Relationship> = serde_json::from_str(&json_str)?;
    Ok(relationships)
}

pub(super) fn write_relationships(
    file_name: &str,
    relationships: &[Relationship],
) -> Result<(), Box<dyn Error>> {
    let json_str = serde_json::to_string_pretty(relationships)?;
    fs::write(file_name, json_str)?;
    Ok(())
}

pub(super) fn read_persons(file_name: &str) -> Result<Vec<Person>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let persons: Vec<Person> = serde_json::from_str(&json_str)?;
    Ok(persons)
}

pub(super) fn write_persons(file_name: &str, persons: &[Person]) -> Result<(), Box<dyn Error>> {
    let json_str = serde_json::to_string_pretty(persons)?;
    fs::write(file_name, json_str)?;
    Ok(())
}

pub(super) fn export_puml(
    file_name: &str,
    relationships: &[Relationship],
    persons: &[Person],
) -> Result<(), Box<dyn Error>> {
    let mut text = "@startuml\n".to_string();
    text += "top to bottom direction\n";
    persons.iter().for_each(|person| {
        let mut name = String::new();
        if let Some(info) = &person.info {
            name += info.first_name.as_str();
            if let Some(last_name) = &info.last_name {
                name += " ";
                name += last_name.as_str();
            }
        } else {
            name += "Unknown"
        }
        text += format!("object \"{}\" as p{}\n", name, person.id).as_str()
    });
    relationships
        .iter()
        .filter(|rel| rel.parents().len() > 0 || rel.children.len() > 1)
        .for_each(|rel| text += format!("diamond r{}\n", rel.id).as_str());
    relationships
        .iter()
        .filter(|rel| rel.parents().len() > 0 || rel.children.len() > 1)
        .for_each(|rel| {
            rel.parents()
                .iter()
                .for_each(|parent| text += format!("p{} -- r{}\n", parent, rel.id).as_str());
            rel.children.iter().for_each(|child| {
                text += format!("p{} <-u- r{}\n", child, rel.id).as_str();
            });
        });
    text += "@enduml\n";
    fs::write(file_name, text)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tree::graph;

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
            id: 0,
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
        let test_persons = vec![Person { id: 0, info: None }];
        compare_persons_to_file(test_persons, "test/single_person.json")
    }

    #[test]
    fn unique_persons() -> Result<(), Box<dyn Error>> {
        let relationships = read_relationships("test/unique_persons.json")?;
        let unique_persons = graph::extract_persons(&relationships);
        let test_persons = vec![0, 1];
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

    #[test]
    fn test_write_persons() {
        let mut persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(0, persons[0].id);
        persons[0].id = 1;
        write_persons("test/write_persons.json", &persons).unwrap();
        let mut persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(1, persons[0].id);
        persons[0].id = 0;
        write_persons("test/write_persons.json", &persons).unwrap();
        let persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(0, persons[0].id);
    }

    #[test]
    fn test_write_relationships() {
        let mut relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(0, relationships[0].id);
        relationships[0].id = 1;
        write_relationships("test/write_relationships.json", &relationships).unwrap();
        let mut relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(1, relationships[0].id);
        relationships[0].id = 0;
        write_relationships("test/write_relationships.json", &relationships).unwrap();
        let relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(0, relationships[0].id);
    }
}
