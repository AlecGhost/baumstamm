use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error")]
    Io(#[from] std::io::Error),
    #[error("Serialization error")]
    Serialization(#[from] serde_json::Error),
    #[error("Consistency error")]
    Consistency(#[from] ConsistencyError),
    #[error("Input error")]
    Input(#[from] InputError),
}

#[derive(Debug, Error)]
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

#[derive(Debug, Error)]
pub enum InputError {
    #[error("Person does not exist")]
    PersonDoesNotExist,
    #[error("Invalid relationship id")]
    InvalidRelationshipId,
    #[error("Invalid person id")]
    InvalidPersonId,
    #[error("Cannot add another parent")]
    AlreadyTwoParents,
}
