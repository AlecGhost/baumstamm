use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("Consistency error")]
    Consistency(#[from] ConsistencyError),
    #[error("Input error")]
    Input(#[from] InputError),
    #[error("Display error")]
    Display(#[from] DisplayError),
}

impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Error::Serialization(err) => serializer.serialize_str(&err.to_string()),
            Error::Consistency(err) => err.serialize(serializer),
            Error::Input(err) => err.serialize(serializer),
            Error::Display(err) => err.serialize(serializer),
        }
    }
}

#[derive(Debug, Error, Serialize)]
pub enum ConsistencyError {
    #[error("The number of persons differs")]
    DifferentNumberOfPersons,
    #[error("Relationships and persons do not match")]
    UnmatchedQuantity,
    #[error("More than one relationship with the same id")]
    RelationshipIdExists,
    #[error("Self referencing relationship")]
    SelfReference,
    #[error("A Child cannot be its parent")]
    DirectCycle,
    #[error("Every person must be child of a relationship")]
    MustBeChild,
    #[error("A Person is child of more than one relationship")]
    MoreThanOnceChild,
    #[error("Not all nodes are connected")]
    Unconnected,
    #[error("Cycle in family tree")]
    IndirectCycle,
    #[error("Multiple persons with the same id")]
    PersonIdExists,
}

#[derive(Debug, Error, Serialize)]
pub enum InputError {
    #[error("Invalid relationship id")]
    InvalidRelationshipId,
    #[error("Invalid person id")]
    InvalidPersonId,
    #[error("Key is not present")]
    InvalidKey,
    #[error("No information to remove")]
    NoInfo,
    #[error("Cannot add another parent")]
    AlreadyTwoParents,
}

#[derive(Debug, Error, Serialize)]
pub enum DisplayError {
    #[error("Invalid starting relationship id")]
    InvalidStartId,
    #[error("Invalid retain relationship id")]
    InvalidRetainId,
    #[error("Invalid retaining edge")]
    InvalidRetainEdge,
    #[error("Conflicting retaining edge options")]
    ConflictingRetain,
}
