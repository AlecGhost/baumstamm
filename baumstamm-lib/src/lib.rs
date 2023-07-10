use error::InputError;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use tree::FamilyTree;
use uuid::Uuid;

mod consistency;
pub mod error;
pub mod graph;
mod io;
mod tree;

pub type PersonInfo = HashMap<String, String>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PersonId(pub u128);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RelationshipId(pub u128);

impl Serialize for PersonId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:X}", self.0))
    }
}

impl<'de> Deserialize<'de> for PersonId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        u128::from_str_radix(s, 16)
            .map(PersonId)
            .map_err(serde::de::Error::custom)
    }
}

impl Serialize for RelationshipId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:X}", self.0))
    }
}

impl<'de> Deserialize<'de> for RelationshipId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = Deserialize::deserialize(deserializer)?;
        u128::from_str_radix(s, 16)
            .map(RelationshipId)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relationship {
    pub id: RelationshipId,
    pub parents: [Option<PersonId>; 2],
    pub children: Vec<PersonId>,
}

impl Relationship {
    fn new(p1: Option<PersonId>, p2: Option<PersonId>, children: Vec<PersonId>) -> Self {
        Self {
            id: RelationshipId(Uuid::new_v4().to_u128_le()),
            parents: [p1, p2],
            children,
        }
    }

    fn parents(&self) -> Vec<PersonId> {
        self.parents.iter().filter_map(|parent| *parent).collect()
    }

    fn add_parent(&mut self) -> Result<Person, InputError> {
        if self.parents().len() == 2 {
            return Err(InputError::AlreadyTwoParents);
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

    fn descendants(&self, relationships: &[Self]) -> Vec<PersonId> {
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub info: Option<PersonInfo>,
}

impl Person {
    fn new() -> Self {
        Self {
            id: PersonId(Uuid::new_v4().to_u128_le()),
            info: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}
