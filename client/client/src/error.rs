use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error("event not found")]
    EventNotFound,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
}
