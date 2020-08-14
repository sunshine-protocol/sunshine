use thiserror::Error;

#[derive(Debug, Error)]
#[error("Vote percent threshold input must be 0 < x < 100")]
pub struct VotePercentThresholdInputBoundError;

#[derive(Debug, Error)]
#[error("Input error for posting bounty.")]
pub struct PostBountyInputError;

#[derive(Debug, Error)]
#[error("Invalid Github Issue Url.")]
pub struct InvalidGithubIssueUrl;
