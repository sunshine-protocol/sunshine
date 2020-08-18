use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("encoded issue exceeds buffer for issue hash")]
    EncodedIssueExceededBuffer,
    #[error("event not found")]
    EventNotFound,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
}
