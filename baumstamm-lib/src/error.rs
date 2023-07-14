use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum Error {
    #[serde(serialize_with = "serialize_serde_error")]
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Consistency error: {0}")]
    Consistency(#[from] ConsistencyError),
    #[error("Input error: {0}")]
    Input(#[from] InputError),
    #[error("Display error: {0}")]
    Display(#[from] DisplayError),
}

fn serialize_serde_error<S>(error: &serde_json::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&error.to_string())
}

#[derive(Debug, Error, Serialize)]
pub enum ConsistencyError {
    #[error("The number of persons differs")]
    DifferentNumberOfPersons,
    #[error("Relationships and persons do not match")]
    UnmatchedQuantity,
    #[error("More than one relationship with the same id")]
    RelationshipIdExists,
    #[error("More than one relationship with the same parents")]
    RelationshipExists,
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
