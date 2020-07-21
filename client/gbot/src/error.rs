use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error("Other error")]
    Other,
}

pub type Result<T> = core::result::Result<T, Error>;
