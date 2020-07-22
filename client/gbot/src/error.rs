use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    NoGithubToken(#[from] std::env::VarError),
}

pub type Result<T> = core::result::Result<T, Error>;
