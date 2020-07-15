use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<E: std::fmt::Debug + std::error::Error + 'static> {
    #[error(transparent)]
    Client(E),
    #[error(transparent)]
    InvalidSs58(#[from] sunshine_core::InvalidSs58),
    #[error("Vote percent threshold input must be 0 < x < 100")]
    VotePercentThresholdInputBoundError,
    #[error("Input error for posting bounty.")]
    PostBountyInputError,
}

pub type Result<T, E> = core::result::Result<T, Error<E>>;
