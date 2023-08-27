use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(untagged)]
pub enum Error {
    #[serde(serialize_with = "serialize_to_string")]
    #[error(transparent)]
    Baumstamm(#[from] baumstamm_lib::error::Error),
    #[serde(serialize_with = "serialize_to_string")]
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Command(#[from] CommandError),
}

#[derive(Debug, Error, Serialize)]
pub enum CommandError {
    #[error("Not in a workspace")]
    NoWorkspace,
}

fn serialize_to_string<E: ToString, S>(error: &E, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&error.to_string())
}
