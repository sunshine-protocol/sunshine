use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("event not found")]
    EventNotFound,
    #[error("Custom description cannot be parsed from string")]
    ParseCodecError,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
}
