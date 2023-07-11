use crate::{error::Error, TreeData};

pub(super) fn read(json_str: &str) -> Result<TreeData, Error> {
    let tree_data = serde_json::from_str(json_str)?;
    Ok(tree_data)
}

pub(super) fn write(tree_data: &TreeData) -> Result<String, Error> {
    let json_str = serde_json::to_string_pretty(tree_data)?;
    Ok(json_str)
}

#[cfg(test)]
mod test {
    use crate::{extract_persons, Person, PersonId, Relationship, RelationshipId, TreeData};
    use std::{error::Error, fs};

    fn read(file_name: &str) -> Result<TreeData, Box<dyn Error>> {
        let json_data = fs::read_to_string(file_name)?;
        let tree_data = super::read(&json_data)?;
        Ok(tree_data)
    }

    fn write(file_name: &str, tree_data: &TreeData) -> Result<(), Box<dyn Error>> {
        let json_str = super::write(tree_data)?;
        fs::write(file_name, json_str)?;
        Ok(())
    }

    fn compare_rels_to_file(
        test_rels: Vec<Relationship>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let tree_data = read(file_name)?;
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
            id: RelationshipId(0),
            parents: [None, None],
            children: Vec::new(),
        }];
        compare_rels_to_file(test_rels, "test/io/single_rel.json")
    }

    fn compare_persons_to_file(
        test_persons: Vec<Person>,
        file_name: &str,
    ) -> Result<(), Box<dyn Error>> {
        let tree_data = read(file_name)?;
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
        let test_persons = vec![Person {
            id: PersonId(0),
            info: None,
        }];
        compare_persons_to_file(test_persons, "test/io/single_person.json")
    }

    #[test]
    fn unique_persons() -> Result<(), Box<dyn Error>> {
        let tree_data = read("test/io/unique_persons.json")?;
        let unique_persons = extract_persons(&tree_data.relationships);
        let test_persons = vec![PersonId(0), PersonId(1)];
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
    fn test_write_persons() -> Result<(), Box<dyn Error>> {
        let mut tree_data = read("test/io/write_persons.json")?;
        assert_eq!(PersonId(0), tree_data.persons[0].id);
        tree_data.persons[0].id = PersonId(1);
        write("test/io/write_persons.json", &tree_data)?;
        let mut tree_data = read("test/io/write_persons.json")?;
        assert_eq!(PersonId(1), tree_data.persons[0].id);
        tree_data.persons[0].id = PersonId(0);
        write("test/io/write_persons.json", &tree_data)?;
        let tree_data = read("test/io/write_persons.json")?;
        assert_eq!(PersonId(0), tree_data.persons[0].id);
        Ok(())
    }

    #[test]
    fn test_write_relationships() -> Result<(), Box<dyn Error>> {
        let mut tree_data = read("test/io/write_relationships.json")?;
        assert_eq!(RelationshipId(0), tree_data.relationships[0].id);
        tree_data.relationships[0].id = RelationshipId(1);
        write("test/io/write_relationships.json", &tree_data)?;
        let mut tree_data = read("test/io/write_relationships.json")?;
        assert_eq!(RelationshipId(1), tree_data.relationships[0].id);
        tree_data.relationships[0].id = RelationshipId(0);
        write("test/io/write_relationships.json", &tree_data)?;
        let tree_data = read("test/io/write_relationships.json")?;
        assert_eq!(RelationshipId(0), tree_data.relationships[0].id);
        Ok(())
    }
}
