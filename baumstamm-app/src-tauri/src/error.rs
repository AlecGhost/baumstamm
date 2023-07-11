use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
pub enum Error {
    #[error(transparent)]
    Baumstamm(#[from] baumstamm_lib::error::Error),
    #[serde(serialize_with = "serialize_io_error")]
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn serialize_io_error<S>(error: &std::io::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&error.to_string())
}
