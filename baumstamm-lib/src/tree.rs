use itertools::Itertools;
use std::{collections::HashMap, error::Error};
use uuid::Uuid;

mod consistency;
mod graph;
mod io;

pub type PersonId = u128;
pub type RelationshipId = u128;
pub type PersonInfo = HashMap<String, String>;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Relationship {
    pub id: RelationshipId,
    pub parents: [Option<PersonId>; 2],
    pub children: Vec<PersonId>,
}

impl Relationship {
    fn new(p1: Option<PersonId>, p2: Option<PersonId>, children: Vec<PersonId>) -> Self {
        Self {
            id: Uuid::new_v4().to_u128_le(),
            parents: [p1, p2],
            children,
        }
    }

    fn parents(&self) -> Vec<PersonId> {
        self.parents.iter().filter_map(|parent| *parent).collect()
    }

    fn add_parent(&mut self) -> Result<Person, &'static str> {
        if self.parents().len() == 2 {
            return Err("Cannot add another parent");
        }
        let parent = Person::new();
        if self.parents[0].is_none() {
            self.parents[0] = Some(parent.id);
        } else {
            self.parents[1] = Some(parent.id);
        }
        Ok(parent)
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

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub info: Option<PersonInfo>,
}

impl Person {
    fn new() -> Self {
        Person {
            id: Uuid::new_v4().to_u128_le(),
            info: None,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TreeData {
    relationships: Vec<Relationship>,
    persons: Vec<Person>,
}

impl TreeData {
    fn new(relationships: Vec<Relationship>, persons: Vec<Person>) -> Self {
        Self {
            relationships,
            persons,
        }
    }
}

pub struct FamilyTree {
    file_name: String,
    tree_data: TreeData,
}

impl FamilyTree {
    pub fn new(file_name: String) -> Result<Self, Box<dyn Error>> {
        let initial_person = Person::new();
        let initial_rels = vec![Relationship::new(None, None, vec![initial_person.id])];
        let tree = FamilyTree {
            file_name,
            tree_data: TreeData::new(initial_rels, vec![initial_person]),
        };
        tree.save()?;

        Ok(tree)
    }

    pub fn from_disk(file_name: String) -> Result<Self, Box<dyn Error>> {
        let tree_data = io::read(&file_name)?;
        consistency::check(&tree_data)?;

        Ok(FamilyTree {
            file_name,
            tree_data,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        consistency::check(&self.tree_data)?;
        io::write(&self.file_name, &self.tree_data)?;
        Ok(())
    }

    pub fn get_persons(&self) -> &[Person] {
        self.tree_data.persons.as_slice()
    }

    pub fn add_parent(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<(PersonId, RelationshipId), Box<dyn Error>> {
        let rel_opt = self
            .tree_data
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);
        let rel = match rel_opt {
            Some(rel) => rel,
            None => return Err("Invalid relationship id".into()),
        };

        let parent = rel.add_parent()?;
        let new_pid = parent.id;

        let new_rel = Relationship::new(None, None, vec![parent.id]);
        let new_rid = new_rel.id;

        self.tree_data.persons.push(parent);
        self.tree_data.relationships.push(new_rel);
        self.save()?;

        Ok((new_pid, new_rid))
    }

    pub fn add_child(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<PersonId, Box<dyn Error>> {
        let rel_opt = self
            .tree_data
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);

        let rel = match rel_opt {
            Some(rel) => rel,
            None => return Err("Invalid relationship id".into()),
        };
        let new_person = Person::new();
        let new_id = new_person.id;
        self.tree_data.persons.push(new_person);
        rel.children.push(new_id);
        self.save()?;

        Ok(new_id)
    }

    pub fn add_rel(&mut self, person_id: PersonId) -> Result<RelationshipId, Box<dyn Error>> {
        if !self
            .tree_data
            .persons
            .iter()
            .map(|person| person.id)
            .any(|id| id == person_id)
        {
            return Err("Person does not exist.".into());
        }
        let new_rel = Relationship::new(Some(person_id), None, vec![]);
        let new_rid = new_rel.id;
        self.tree_data.relationships.push(new_rel);
        self.save()?;

        Ok(new_rid)
    }

    pub fn add_rel_with_partner(
        &mut self,
        person_id: PersonId,
        partner_id: PersonId,
    ) -> Result<RelationshipId, Box<dyn Error>> {
        if !self
            .tree_data
            .persons
            .iter()
            .map(|person| person.id)
            .any(|id| id == person_id)
            || !self
                .tree_data
                .persons
                .iter()
                .map(|person| person.id)
                .any(|id| id == partner_id)
        {
            return Err("Person does not exist.".into());
        }
        let new_rel = Relationship::new(Some(person_id), Some(partner_id), vec![]);
        let new_rid = new_rel.id;
        self.tree_data.relationships.push(new_rel);
        self.save()?;

        Ok(new_rid)
    }

    pub fn add_info(
        &mut self,
        person_id: PersonId,
        person_info: Option<PersonInfo>,
    ) -> Result<(), Box<dyn Error>> {
        let person = match self
            .tree_data
            .persons
            .iter_mut()
            .find(|person| person.id == person_id)
        {
            Some(person) => person,
            None => return Err("Invalid person id".into()),
        };
        person.info = person_info;
        self.save()?;

        Ok(())
    }
}
