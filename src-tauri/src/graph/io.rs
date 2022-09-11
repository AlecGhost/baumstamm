use std::{error::Error, fs};

use super::{Person, Relationship};

pub fn read_relationships(file_name: &str) -> Result<Vec<Relationship>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let relationships: Vec<Relationship> = serde_json::from_str(&json_str)?;
    super::consistency::check_relationships(&relationships)?;
    Ok(relationships)
}

pub fn write_relationships(
    file_name: &str,
    relationships: &[Relationship],
) -> Result<(), Box<dyn Error>> {
    let json_str = serde_json::to_string_pretty(relationships)?;
    fs::write(file_name, json_str)?;
    Ok(())
}

pub fn read_persons(file_name: &str) -> Result<Vec<Person>, Box<dyn Error>> {
    let json_str = fs::read_to_string(file_name)?;
    let persons: Vec<Person> = serde_json::from_str(&json_str)?;
    super::consistency::check_persons(&persons)?;
    Ok(persons)
}

pub fn write_persons(file_name: &str, persons: &[Person]) -> Result<(), Box<dyn Error>> {
    let json_str = serde_json::to_string_pretty(persons)?;
    fs::write(file_name, json_str)?;
    Ok(())
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
            id: uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
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
            id: uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            info: None,
        }];
        compare_persons_to_file(test_persons, "test/single_person.json")
    }

    #[test]
    fn unique_persons() -> Result<(), Box<dyn Error>> {
        let relationships = read_relationships("test/unique_persons.json")?;
        let unique_persons = crate::graph::extract_persons(&relationships);
        let test_persons = vec![
            uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8"),
            uuid::uuid!("57e55044-10b1-426f-9247-bb680e5fe0c8"),
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

    #[test]
    fn test_write_persons() {
        let mut persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(
            "67e55044-10b1-426f-9247-bb680e5fe0c8",
            persons[0].id.to_string()
        );
        persons[0].id = uuid::uuid!("57e55044-10b1-426f-9247-bb680e5fe0c8");
        write_persons("test/write_persons.json", &persons).unwrap();
        let mut persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(
            "57e55044-10b1-426f-9247-bb680e5fe0c8",
            persons[0].id.to_string()
        );
        persons[0].id = uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");
        write_persons("test/write_persons.json", &persons).unwrap();
        let persons = read_persons("test/write_persons.json").unwrap();
        assert_eq!(
            "67e55044-10b1-426f-9247-bb680e5fe0c8",
            persons[0].id.to_string()
        );
    }

    #[test]
    fn test_write_relationships() {
        let mut relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(
            "67e55044-10b1-426f-9247-bb680e5fe0c8",
            relationships[0].id.to_string()
        );
        relationships[0].id = uuid::uuid!("57e55044-10b1-426f-9247-bb680e5fe0c8");
        write_relationships("test/write_relationships.json", &relationships).unwrap();
        let mut relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(
            "57e55044-10b1-426f-9247-bb680e5fe0c8",
            relationships[0].id.to_string()
        );
        relationships[0].id = uuid::uuid!("67e55044-10b1-426f-9247-bb680e5fe0c8");
        write_relationships("test/write_relationships.json", &relationships).unwrap();
        let relationships = read_relationships("test/write_relationships.json").unwrap();
        assert_eq!(
            "67e55044-10b1-426f-9247-bb680e5fe0c8",
            relationships[0].id.to_string()
        );
    }
}
