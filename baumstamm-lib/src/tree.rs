use crate::{
    consistency,
    error::{Error, InputError},
    io, Person, PersonId, Relationship, RelationshipId, TreeData,
};
use specta::Type;
use std::collections::HashMap;

#[derive(Debug, Type)]
pub struct FamilyTree {
    tree_data: TreeData,
}

impl FamilyTree {
    pub fn new() -> Self {
        let initial_person = Person::new();
        let initial_rels = vec![Relationship::new(None, None, vec![initial_person.id])];
        Self {
            tree_data: TreeData::new(initial_rels, vec![initial_person]),
        }
    }

    pub fn from_string(json_str: &str) -> Result<Self, Error> {
        let tree_data = io::read(json_str)?;
        consistency::check(&tree_data)?;
        Ok(Self { tree_data })
    }

    pub fn save(&self) -> Result<String, Error> {
        consistency::check(&self.tree_data)?;
        io::write(&self.tree_data)
    }

    pub fn get_persons(&self) -> &[Person] {
        self.tree_data.persons.as_slice()
    }

    pub fn get_relationships(&self) -> &[Relationship] {
        self.tree_data.relationships.as_slice()
    }

    pub fn add_parent(
        &mut self,
        relationship_id: RelationshipId,
    ) -> Result<(PersonId, RelationshipId), Error> {
        let rel_opt = self
            .tree_data
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);
        let rel = match rel_opt {
            Some(rel) => rel,
            None => return Err(InputError::InvalidRelationshipId.into()),
        };

        let parent = rel.add_parent()?;
        let new_pid = parent.id;

        let new_rel = Relationship::new(None, None, vec![parent.id]);
        let new_rid = new_rel.id;

        self.tree_data.persons.push(parent);
        self.tree_data.relationships.push(new_rel);
        if let Err(err) = consistency::check(&self.tree_data) {
            self.tree_data.persons.pop();
            self.tree_data.relationships.pop();
            return Err(err.into());
        }

        Ok((new_pid, new_rid))
    }

    pub fn add_child(&mut self, relationship_id: RelationshipId) -> Result<PersonId, Error> {
        let rel_opt = self
            .tree_data
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id);

        let rel = rel_opt.ok_or(InputError::InvalidRelationshipId)?;
        let new_person = Person::new();
        let new_id = new_person.id;
        self.tree_data.persons.push(new_person);
        rel.children.push(new_id);
        if let Err(err) = consistency::check(&self.tree_data) {
            self.tree_data.persons.pop();
            return Err(err.into());
        }

        Ok(new_id)
    }

    fn validate_person(&self, person_id: PersonId) -> Result<(), Error> {
        if self
            .tree_data
            .persons
            .iter()
            .map(|person| person.id)
            .any(|id| id == person_id)
        {
            Ok(())
        } else {
            Err(InputError::InvalidPersonId.into())
        }
    }

    pub fn add_new_relationship(&mut self, person_id: PersonId) -> Result<RelationshipId, Error> {
        self.validate_person(person_id)?;
        let new_rel = Relationship::new(Some(person_id), None, vec![]);
        let new_rid = new_rel.id;
        self.tree_data.relationships.push(new_rel);
        if let Err(err) = consistency::check(&self.tree_data) {
            self.tree_data.relationships.pop();
            return Err(err.into());
        }

        Ok(new_rid)
    }

    pub fn add_relationship_with_partner(
        &mut self,
        person_id: PersonId,
        partner_id: PersonId,
    ) -> Result<RelationshipId, Error> {
        self.validate_person(person_id)?;
        self.validate_person(partner_id)?;
        let new_rel = Relationship::new(Some(person_id), Some(partner_id), Vec::new());
        let new_rid = new_rel.id;
        self.tree_data.relationships.push(new_rel);
        if let Err(err) = consistency::check(&self.tree_data) {
            self.tree_data.relationships.pop();
            return Err(err.into());
        }

        Ok(new_rid)
    }

    pub fn insert_info(
        &mut self,
        person_id: PersonId,
        key: String,
        value: String,
    ) -> Result<(), Error> {
        let person = self
            .tree_data
            .persons
            .iter_mut()
            .find(|person| person.id == person_id)
            .ok_or(InputError::InvalidPersonId)?;
        if let Some(info) = &mut person.info {
            info.insert(key, value);
        } else {
            person.info = Some(HashMap::from([(key, value)]));
        }

        Ok(())
    }

    pub fn remove_info(&mut self, person_id: PersonId, key: &str) -> Result<String, Error> {
        let person = self
            .tree_data
            .persons
            .iter_mut()
            .find(|person| person.id == person_id)
            .ok_or(InputError::InvalidPersonId)?;
        let info = person.info.as_mut().ok_or(InputError::NoInfo)?;
        let value = info.remove(key).ok_or(InputError::InvalidKey)?;

        Ok(value)
    }
}

impl Default for FamilyTree {
    fn default() -> Self {
        Self::new()
    }
}
