use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("event not found")]
    EventNotFound,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
    #[error("Vote percent threshold input must be 0 < x < 100")]
    VotePercentThresholdInputBoundError,
}
