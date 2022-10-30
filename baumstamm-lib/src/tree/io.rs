use std::{error::Error, fs};

use super::TreeData;

pub(super) fn read(file_name: &str) -> Result<TreeData, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let tree_data: TreeData = serde_json::from_str(&json_str)?;
    Ok(tree_data)
}

pub(super) fn write(file_name: &str, tree_data: &TreeData) -> Result<(), Box<dyn Error>> {
    let json_str = serde_json::to_string_pretty(tree_data)?;
    fs::write(file_name, json_str)?;
    Ok(())
}

pub(super) fn export_puml(file_name: &str, tree_data: &TreeData) -> Result<(), Box<dyn Error>> {
    let mut text = "@startuml\n".to_string();
    text += "top to bottom direction\n";
    tree_data.persons.iter().for_each(|person| {
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
    tree_data
        .relationships
        .iter()
        .filter(|rel| !rel.parents().is_empty() || rel.children.len() > 1)
        .for_each(|rel| text += format!("diamond r{}\n", rel.id).as_str());
    tree_data
        .relationships
        .iter()
        .filter(|rel| !rel.parents().is_empty() || rel.children.len() > 1)
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
    use crate::tree::{graph, Person, Relationship};

    fn compare_rels_to_file(
        test_rels: Vec<Relationship>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let tree_data = match read(file_name) {
            Ok(value) => value,
            Err(_) => return Err("Deserialization failed".into()),
        };
        if tree_data.relationships == test_rels {
            Ok(())
        } else {
            let message = format!(
                "Json does not match\nInput: {:?}\nExpected: {:?}",
                tree_data.relationships, test_rels
            );
            Err(message.into())
        }
    }

    #[test]
    fn empty_rels() -> Result<(), Box<dyn Error>> {
        compare_rels_to_file(Vec::new(), "test/io/empty.json")
    }

    #[test]
    fn single_rel() -> Result<(), Box<dyn Error>> {
        let test_rels = vec![Relationship {
            id: 0,
            parents: [None, None],
            children: Vec::new(),
        }];
        compare_rels_to_file(test_rels, "test/io/single_rel.json")
    }

    fn compare_persons_to_file(
        test_persons: Vec<Person>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let tree_data = match read(file_name) {
            Ok(value) => value,
            Err(_) => return Err("Deserialization failed".into()),
        };
        if tree_data.persons == test_persons {
            Ok(())
        } else {
            let message = format!(
                "Json does not match\nInput: {:?}\nExpected: {:?}",
                tree_data.persons, test_persons
            );
            Err(message.into())
        }
    }

    #[test]
    fn empty_persons() -> Result<(), Box<dyn Error>> {
        compare_persons_to_file(Vec::new(), "test/io/empty.json")
    }

    #[test]
    fn single_person() -> Result<(), Box<dyn Error>> {
        let test_persons = vec![Person { id: 0, info: None }];
        compare_persons_to_file(test_persons, "test/io/single_person.json")
    }

    #[test]
    fn unique_persons() -> Result<(), Box<dyn Error>> {
        let tree_data = read("test/io/unique_persons.json")?;
        let unique_persons = graph::extract_persons(&tree_data.relationships);
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
        let mut tree_data = read("test/io/write_persons.json").unwrap();
        assert_eq!(0, tree_data.persons[0].id);
        tree_data.persons[0].id = 1;
        write("test/io/write_persons.json", &tree_data).unwrap();
        let mut tree_data = read("test/io/write_persons.json").unwrap();
        assert_eq!(1, tree_data.persons[0].id);
        tree_data.persons[0].id = 0;
        write("test/io/write_persons.json", &tree_data).unwrap();
        let tree_data = read("test/io/write_persons.json").unwrap();
        assert_eq!(0, tree_data.persons[0].id);
    }

    #[test]
    fn test_write_relationships() {
        let mut tree_data = read("test/io/write_relationships.json").unwrap();
        assert_eq!(0, tree_data.relationships[0].id);
        tree_data.relationships[0].id = 1;
        write("test/io/write_relationships.json", &tree_data).unwrap();
        let mut tree_data = read("test/io/write_relationships.json").unwrap();
        assert_eq!(1, tree_data.relationships[0].id);
        tree_data.relationships[0].id = 0;
        write("test/io/write_relationships.json", &tree_data).unwrap();
        let tree_data = read("test/io/write_relationships.json").unwrap();
        assert_eq!(0, tree_data.relationships[0].id);
    }
}