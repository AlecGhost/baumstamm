use crate::{
    consistency,
    error::{Error, InputError, MergeConflict},
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
            let children_empty = rel.children.is_empty();
            let parent_count = rel.parents.iter().flatten().count();
            let empty = parent_count <= 1 && children_empty;
            // delete rel if it is now empty (even if the partner is still there)
            !empty
        });
        if consistency::check(&self.tree_data).is_err() {
            // restore old state
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

    pub fn merge_person(
        &mut self,
        person_id1: PersonId,
        person_id2: PersonId,
    ) -> Result<(), Error> {
        fn merge_info(
            person1: &Person,
            person2: &Person,
        ) -> Result<Option<HashMap<String, String>>, MergeConflict> {
            match (&person1.info, &person2.info) {
                (Some(info1), Some(info2)) => {
                    let mut new_info = info1.clone();
                    for (k2, v2) in info2 {
                        use std::collections::hash_map::Entry;
                        match new_info.entry(k2.to_string()) {
                            Entry::Occupied(entry) => {
                                if entry.get() != v2 {
                                    return Err(MergeConflict::DifferentInfo);
                                }
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(v2.to_string());
                            }
                        };
                    }
                    Ok(Some(new_info))
                }
                (Some(info), None) => Ok(Some(info.clone())),
                (None, Some(info)) => Ok(Some(info.clone())),
                (None, None) => Ok(None),
            }
        }

        fn merge_parents(
            rel1: &Relationship,
            rel2: &Relationship,
        ) -> Result<[Option<PersonId>; 2], MergeConflict> {
            match (rel1.parents, rel2.parents) {
                // no parent
                ([None, None], [None, None]) => Ok([None, None]),
                // one parent
                ([Some(parent), None], [None, None]) => Ok([Some(parent), None]),
                ([None, Some(parent)], [None, None]) => Ok([Some(parent), None]),
                ([None, None], [Some(parent), None]) => Ok([Some(parent), None]),
                ([None, None], [None, Some(parent)]) => Ok([Some(parent), None]),
                // two parents
                ([Some(parent1), Some(parent2)], [None, None]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                ([None, None], [Some(parent1), Some(parent2)]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                ([Some(parent1), None], [None, Some(parent2)]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                ([Some(parent1), None], [Some(parent2), None]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                ([None, Some(parent1)], [None, Some(parent2)]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                ([None, Some(parent1)], [Some(parent2), None]) => {
                    Ok([Some(parent1), Some(parent2)])
                }
                _ => Err(MergeConflict::TooManyParents),
            }
        }

        fn merge_siblings(
            rel1: &Relationship,
            rel2: &Relationship,
            person_id1: PersonId,
            person_id2: PersonId,
        ) -> Vec<PersonId> {
            let mut siblings = Vec::new();
            siblings.extend(
                rel1.children
                    .iter()
                    .cloned()
                    .filter(|child| *child != person_id1)
                    .collect_vec(),
            );
            siblings.extend(
                rel2.children
                    .iter()
                    .cloned()
                    .filter(|child| *child != person_id2)
                    .collect_vec(),
            );
            siblings
        }

        if person_id1 == person_id2 {
            return Err(InputError::SelfMerge.into());
        }

        let (pos1, person1) = self.find_pos_and_person(person_id1)?;
        let (pos2, person2) = self.find_pos_and_person(person_id2)?;
        let merged_info = merge_info(person1, person2)?;

        let persons = &mut self.tree_data.persons;
        let rels = &mut self.tree_data.relationships;
        let rel1 = rels
            .iter()
            .find(|rel| rel.children.contains(&person_id1))
            .expect("Rel must exist");
        let rel2 = rels
            .iter()
            .find(|rel| rel.children.contains(&person_id2))
            .expect("Rel must exist");

        let merged_parents = merge_parents(rel1, rel2)?;
        let mut merged_siblings = merge_siblings(rel1, rel2, person_id1, person_id2);

        // create new person and parent rel
        let mut new_person = Person::new();
        new_person.info = merged_info;
        merged_siblings.push(new_person.id);
        let new_rel = Relationship::new(merged_parents[0], merged_parents[1], merged_siblings);

        // sort positions
        let (pos1, pos2) = if pos1 < pos2 {
            (pos1, pos2)
        } else {
            (pos2, pos1)
        };

        // backup for restoration after failed consistency check
        let backup_person1 = persons.remove(pos1);
        let backup_person2 = persons.remove(pos2 - 1);
        let mut backup_rels = Vec::new();

        // edit rels
        rels.retain(|rel| {
            if rel.children.contains(&person_id1) || rel.children.contains(&person_id2) {
                backup_rels.push(rel.clone());
                false
            } else {
                true
            }
        });
        for rel in rels.iter_mut() {
            if rel.parents().contains(&person_id1) || rel.parents().contains(&person_id2) {
                backup_rels.push(rel.clone());
                for parent in rel
                    .parents
                    .iter_mut()
                    .filter_map(|parent| parent.as_mut())
                    .filter(|parent| **parent == person_id1 || **parent == person_id2)
                {
                    *parent = new_person.id;
                }
            }
        }

        // add new person and rel
        persons.push(new_person);
        rels.push(new_rel);

        if let Err(err) = consistency::check(&self.tree_data) {
            // restore old state
            let persons = &mut self.tree_data.persons;
            let rels = &mut self.tree_data.relationships;
            persons.pop();
            persons.push(backup_person1);
            persons.push(backup_person2);
            rels.pop();
            rels.retain(|rel| backup_rels.iter().all(|backup_rel| backup_rel.id != rel.id));
            rels.extend(backup_rels);
            return Err(MergeConflict::InconsistentTree(err).into());
        }
        Ok(())
    }

    pub fn insert_info(
        &mut self,
        person_id: PersonId,
        key: String,
        value: String,
    ) -> Result<(), Error> {
        let person = self.find_person_mut(person_id)?;
        if let Some(info) = &mut person.info {
            info.insert(key, value);
        } else {
            person.info = Some(HashMap::from([(key, value)]));
        }

        Ok(())
    }

    pub fn remove_info(&mut self, person_id: PersonId, key: &str) -> Result<String, Error> {
        let person = self.find_person_mut(person_id)?;
        let info = person.info.as_mut().ok_or(InputError::NoInfo)?;
        let value = info.remove(key).ok_or(InputError::InvalidKey)?;

        Ok(value)
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
            Err(InputError::InvalidPersonId)
        }
    }

    fn find_person_mut(&mut self, person_id: PersonId) -> Result<&mut Person, InputError> {
        let person = self
            .tree_data
            .persons
            .iter_mut()
            .find(|person| person.id == person_id)
            .ok_or(InputError::InvalidPersonId)?;
        Ok(person)
    }

    fn find_pos_and_person(&self, person_id: PersonId) -> Result<(usize, &Person), InputError> {
        let result = self
            .tree_data
            .persons
            .iter()
            .enumerate()
            .find(|(_, person)| person.id == person_id)
            .ok_or(InputError::InvalidPersonId)?;
        Ok(result)
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
