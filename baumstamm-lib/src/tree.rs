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
    parents: [Option<PersonId>; 2],
    children: Vec<PersonId>,
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
        self.parents
            .iter()
            .filter_map(|parent| parent.clone())
            .collect()
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

    fn generations_below(&self, relationships: &[Relationship]) -> u32 {
        fn generations_below_recursive(
            relationship: &Relationship,
            relationships: &[Relationship],
            generations_above: u32,
        ) -> u32 {
            relationship
                .children
                .iter()
                .flat_map(|child| graph::parent_relationships(child, relationships))
                .map(|rel| generations_below_recursive(rel, relationships, generations_above + 1))
                .max()
                .unwrap_or_else(|| 0)
            // .expect("Inconsistent data")
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

    pub fn export_puml(&self, file_name: &str) -> Result<(), Box<dyn Error>> {
        consistency::check(&self.relationships, &self.persons)?;
        io::export_puml(file_name, &self.relationships, &self.persons)?;
        Ok(())
    }

    pub fn get_persons(&self) -> &[Person] {
        self.persons.as_slice()
    }

    pub fn add_parent(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<(PersonId, RelationshipId), Box<dyn Error>> {
        let rel_opt = self
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

        self.persons.push(parent);
        self.relationships.push(new_rel);
        self.save()?;

        Ok((new_pid, new_rid))
    }

    pub fn add_child(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<PersonId, Box<dyn Error>> {
        let rel_opt = self
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);

        let rel = match rel_opt {
            Some(rel) => rel,
            None => return Err("Invalid relationship id".into()),
        };
        let new_person = Person::new();
        let new_id = new_person.id;
        self.persons.push(new_person);
        rel.children.push(new_id);
        self.save()?;

        Ok(new_id)
    }

    pub fn add_rel(&mut self, person_id: PersonId) -> Result<RelationshipId, Box<dyn Error>> {
        if !self
            .persons
            .iter()
            .map(|person| person.id)
            .any(|id| id == person_id)
        {
            return Err("Person does not exist.".into());
        }
        let new_rel = Relationship::new(Some(person_id), None, vec![]);
        let new_rid = new_rel.id;
        self.relationships.push(new_rel);
        self.save()?;

        Ok(new_rid)
    }

    pub fn add_rel_with_partner(
        &mut self,
        person_id: PersonId,
        partner_id: PersonId,
    ) -> Result<RelationshipId, Box<dyn Error>> {
        if !self
            .persons
            .iter()
            .map(|person| person.id)
            .any(|id| id == person_id)
            || !self
                .persons
                .iter()
                .map(|person| person.id)
                .any(|id| id == partner_id)
        {
            return Err("Person does not exist.".into());
        }
        let new_rel = Relationship::new(Some(person_id), Some(partner_id), vec![]);
        let new_rid = new_rel.id;
        self.relationships.push(new_rel);
        self.save()?;

        Ok(new_rid)
    }

    pub fn add_info(
        &mut self,
        person_id: PersonId,
        person_info: Option<PersonInfo>,
    ) -> Result<(), Box<dyn Error>> {
        let person = match self
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
