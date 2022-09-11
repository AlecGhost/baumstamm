use crate::grid::PersonInfo;
use crate::util::UniqueIterator;
use std::error::Error;
use uuid::Uuid;

mod consistency;
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
}

fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
struct Person {
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

    pub fn add_parent(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<PersonId, &'static str> {
        let mut rel_opt: Option<&Relationship> = self
            .relationships
            .iter()
            .find(|rel| rel.id == relationship_id);
        let mut rel: &mut &Relationship = match &mut rel_opt {
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
        Ok(new_id)
    }
}
