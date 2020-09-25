mod error;
// export client error type for ../cli
pub use error::Error;
pub mod bank;
pub mod bounty;
pub mod donate;
pub mod org;
pub mod vote;
pub use sunshine_bounty_utils as utils;

use libipld::DagCbor;
use parity_scale_codec::{
    Decode,
    Encode,
};

#[derive(Default, Clone, DagCbor, Encode, Decode)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Debug, Default, Clone, DagCbor, Encode, Decode)]
pub struct GithubIssue {
    pub issue_number: u64,
    pub repo_owner: String,
    pub repo_name: String,
}
