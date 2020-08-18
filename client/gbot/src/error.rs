use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    NoGithubToken(#[from] std::env::VarError),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Contributions update Bounty Issues")]
    MustRefValidBountyIssue,
    #[error("Submission approvals must update Submission Issues")]
    MustRefValidSubmissionIssue,
}

pub type Result<T> = core::result::Result<T, Error>;
