use error::InputError;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::HashMap;
pub use tree::FamilyTree;
use uuid::Uuid;

mod consistency;
pub mod error;
pub mod graph;
mod io;
mod tree;

/// Arbitrary information about a person.
pub type PersonInfo = HashMap<String, String>;

/// UUID for a `Relationship`, stored as u128.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Type)]
pub struct RelationshipId(#[serde(with = "id")] pub u128);

/// A relationship referencing two optional parents and the resulting children.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
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

/// UUID for a `Person`, stored as u128.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash, Type)]
pub struct PersonId(#[serde(with = "id")] pub u128);

/// A person with a unique identifier and arbitrary attached information
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Type)]
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

/// Raw family tree data.
#[derive(Debug, Serialize, Deserialize, Type)]
pub struct TreeData {
    pub relationships: Vec<Relationship>,
    pub persons: Vec<Person>,
}

impl TreeData {
    fn new(relationships: Vec<Relationship>, persons: Vec<Person>) -> Self {
        Self {
            relationships,
            persons,
        }
    }
}

/// Extract all `PersonId`'s referenced in a slice of `Relationship`'s.
fn extract_persons(relationships: &[Relationship]) -> Vec<PersonId> {
    let parents = relationships.iter().flat_map(|rel| rel.parents());
    let children = relationships.iter().flat_map(|rel| rel.children.to_vec());
    parents.chain(children).unique().collect()
}

// Trait impls for `PersonId`
impl From<u128> for PersonId {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl std::fmt::Debug for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl std::fmt::Display for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

// Trait impls for `RelationshipId`
impl From<u128> for RelationshipId {
    fn from(value: u128) -> Self {
        Self(value)
    }
}

impl std::fmt::Debug for RelationshipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

impl std::fmt::Display for RelationshipId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self, f)
    }
}

mod id {
    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{:X}", value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s: &str = serde::Deserialize::deserialize(deserializer)?;
        u128::from_str_radix(s, 16).map_err(serde::de::Error::custom)
    }
}
