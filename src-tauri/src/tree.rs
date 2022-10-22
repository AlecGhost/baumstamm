use crate::util::UniqueIterator;
use std::error::Error;
use uuid::Uuid;

mod consistency;
mod graph;
mod io;

type PersonId = u128;
type RelationshipId = u128;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Relationship {
    id: RelationshipId,
    p1: Option<PersonId>,
    p2: Option<PersonId>,
    children: Vec<PersonId>,
}

impl Relationship {
    fn new(p1: Option<PersonId>, p2: Option<PersonId>, children: Vec<PersonId>) -> Self {
        Self {
            id: Uuid::new_v4().to_u128_le(),
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
            let descendant = descendants[index];
            relationships
                .iter()
                .filter(|rel| rel.parents().contains(&descendant))
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

    fn generations_below(&self, relationships: &[Relationship]) -> u32 {
        fn generations_below_recursive(
            relationship: &Relationship,
            relationships: &[Relationship],
            generations_above: u32,
        ) -> u32 {
            relationship
                .children
                .iter()
                .flat_map(|child| graph::parent_relationships(child.clone(), relationships))
                .map(|rel| generations_below_recursive(rel, relationships, generations_above + 1))
                .max()
                .expect("Inconsistent data")
        }
        generations_below_recursive(self, relationships, 0)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Person {
    id: PersonId,
    info: Option<PersonInfo>,
}

impl Person {
    fn new() -> Self {
        Person {
            id: Uuid::new_v4().to_u128_le(),
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

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PersonInfo {
    first_name: String,
    last_name: Option<String>,
    date_of_birth: Option<String>,
    date_of_death: Option<String>,
}

impl PersonInfo {
    pub fn new(
        first_name: String,
        last_name: Option<String>,
        date_of_birth: Option<String>,
        date_of_death: Option<String>,
    ) -> PersonInfo {
        PersonInfo {
            first_name,
            last_name,
            date_of_birth,
            date_of_death,
        }
    }
}

pub struct FamilyTree {
    relationships_file_name: String,
    persons_file_name: String,
    relationships: Vec<Relationship>,
    persons: Vec<Person>,
}

impl FamilyTree {
    pub fn new(
        relationships_file_name: String,
        persons_file_name: String,
    ) -> Result<Self, Box<dyn Error>> {
        let initial_person = Person::new();
        let initial_rels = vec![Relationship::new(None, None, vec![initial_person.id])];
        let tree = FamilyTree {
            relationships_file_name,
            persons_file_name,
            relationships: initial_rels,
            persons: vec![initial_person],
        };
        tree.save()?;

        Ok(tree)
    }

    pub fn from_disk(
        relationships_file_name: String,
        persons_file_name: String,
    ) -> Result<Self, Box<dyn Error>> {
        let relationships = io::read_relationships(&relationships_file_name)?;
        let persons = io::read_persons(&persons_file_name)?;
        consistency::check(&relationships, &persons)?;

        Ok(FamilyTree {
            relationships_file_name,
            persons_file_name,
            relationships,
            persons,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        consistency::check(&self.relationships, &self.persons)?;
        io::write_relationships(&self.relationships_file_name, &self.relationships)?;
        io::write_persons(&self.persons_file_name, &self.persons)?;
        Ok(())
    }

    pub fn get_persons(&self) -> &[Person] {
        self.persons.as_slice()
    }

    pub fn add_parent(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<PersonId, &'static str> {
        let rel_opt = self
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);
        let mut rel = match rel_opt {
            Some(rel) => rel,
            None => return Err("Invalid relationship id"),
        };
        if rel.parents().len() == 2 {
            return Err("Cannot add another parent");
        }

        let new_person = Person::new();
        let new_id = new_person.id;
        self.persons.push(new_person);
        if rel.p1.is_none() {
            rel.p1 = Some(new_id);
        } else {
            rel.p2 = Some(new_id);
        }
        consistency::check(&self.relationships, &self.persons)?;

        Ok(new_id)
    }

    pub fn add_child(&mut self, relationship_id: RelationshipId) -> Result<PersonId, &'static str> {
        let rel_opt = self
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);

        let rel = match rel_opt {
            Some(rel) => rel,
            None => return Err("Invalid relationship id"),
        };
        let new_person = Person::new();
        let new_id = new_person.id;
        self.persons.push(new_person);
        rel.children.push(new_id);
        consistency::check(&self.relationships, &self.persons)?;

        Ok(new_id)
    }
}
