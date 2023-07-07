use crate::{
    consistency, io, Person, PersonId, PersonInfo, Relationship, RelationshipId, TreeData,
};
use std::error::Error;

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
