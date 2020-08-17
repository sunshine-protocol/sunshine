use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    NoGithubToken(#[from] std::env::VarError),
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseBoolError(#[from] std::str::ParseBoolError),
    #[error("Bounty cannot be parsed from string")]
    ParseBountyError,
    #[error("Submission cannot be parsed from string")]
    ParseSubmissionError,
    #[error("Issues cannot be reused for other bounties or submissions")]
    CannotReuseIssues,
    #[error("Contributions update Bounty Issues")]
    MustRefValidBountyIssue,
    #[error("Submission approvals must update Submission Issues")]
    MustRefValidSubmissionIssue,
    #[error("Passed incorrect bounty id into update bounty")]
    CannotUpdateDifferentBounty,
}

pub type Result<T> = core::result::Result<T, Error>;
