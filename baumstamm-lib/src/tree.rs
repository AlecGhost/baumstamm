use crate::{
    consistency,
    error::{Error, InputError},
    io, Person, PersonId, Relationship, RelationshipId, TreeData,
};
use itertools::Itertools;
use specta::Type;
use std::collections::HashMap;

/// The central datatype, containing **consistent** tree data.
#[derive(Debug, Type)]
pub struct FamilyTree {
    tree_data: TreeData,
}

impl FamilyTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Serialize the tree data to JSON.
    pub fn save(&self) -> Result<String, Error> {
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
        let rel = self
            .tree_data
            .relationships
            .iter_mut()
            .find(|rel| rel.id == relationship_id)
            .ok_or(InputError::InvalidRelationshipId)?;

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

    fn validate_person(&self, person_id: PersonId) -> Result<(), InputError> {
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

    pub fn remove_person(&mut self, person_id: PersonId) -> Result<(), Error> {
        let persons_index = self
            .tree_data
            .persons
            .iter()
            .map(|person| person.id)
            .position(|id| id == person_id)
            .ok_or(InputError::InvalidPersonId)?;
        let backup_person = self.tree_data.persons.remove(persons_index);
        let backup_rels = self
            .tree_data
            .relationships
            .iter()
            .filter(|rel| {
                rel.children.contains(&person_id) || rel.parents.contains(&Some(person_id))
            })
            .cloned()
            .collect_vec();
        self.tree_data.relationships.retain_mut(|rel| {
            rel.parents.iter_mut().for_each(|parent| {
                if matches!(parent, Some(pid) if *pid == person_id) {
                    *parent = None;
                }
            });
            rel.children.retain(|child| *child != person_id);
            // delete rel if it is empty now
            rel.parents.iter().flatten().count() != 0 || !rel.children.is_empty()
        });
        if consistency::check(&self.tree_data).is_err() {
            self.tree_data.persons.push(backup_person);
            for backup_rel in backup_rels {
                if let Some(rel) = self
                    .tree_data
                    .relationships
                    .iter_mut()
                    .find(|rel| rel.id == backup_rel.id)
                {
                    *rel = backup_rel;
                } else {
                    self.tree_data.relationships.push(backup_rel);
                }
            }
            return Err(InputError::CannotRemovePerson.into());
        }
        Ok(())
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
        let initial_person = Person::new();
        let initial_rels = vec![Relationship::new(None, None, vec![initial_person.id])];
        Self {
            tree_data: TreeData::new(initial_rels, vec![initial_person]),
        }
    }
}

impl TryFrom<TreeData> for FamilyTree {
    type Error = Error;

    fn try_from(tree_data: TreeData) -> Result<Self, Self::Error> {
        consistency::check(&tree_data)?;
        Ok(Self { tree_data })
    }
}

impl TryFrom<&str> for FamilyTree {
    type Error = Error;

    fn try_from(json_str: &str) -> Result<Self, Self::Error> {
        let tree_data = io::read(json_str)?;
        consistency::check(&tree_data)?;
        Ok(Self { tree_data })
    }
}

impl TryFrom<&String> for FamilyTree {
    type Error = Error;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<FamilyTree> for TreeData {
    fn from(value: FamilyTree) -> Self {
        value.tree_data
    }
}
