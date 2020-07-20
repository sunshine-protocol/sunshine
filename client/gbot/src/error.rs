use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    Client(#[from] sunshine_bounty_client::Error),
    #[error("event not found")]
    EventNotFound,
}

pub type Result<T> = core::result::Result<T, Error>;
