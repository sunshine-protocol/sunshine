#![allow(clippy::too_many_arguments)]

mod error;
pub use error::Error;
use error::Result;

use octocrab::Octocrab;

#[derive(Debug, Clone)]
pub struct GBot {
    crab: Octocrab,
}

impl GBot {
    pub fn new() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN")?;
        let crab = Octocrab::builder().personal_token(token).build()?;
        Ok(GBot { crab })
    }
}

impl GBot {
    pub async fn issue_comment_bounty_post(
        &self,
        amount: u128,
        bounty_id: u64,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .create_comment(
                issue_number,
                format!(
                    "${} Bounty Posted On Chain, BountyId {}",
                    amount, bounty_id,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn issue_comment_bounty_contribute(
        &self,
        bounty_id: u64,
        total_balance: u128,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .create_comment(
                issue_number,
                format!(
                    "Contribution to Bounty {} increases Total Balance to ${}",
                    bounty_id, total_balance
                ),
            )
            .await?;
        Ok(())
    }
    // TODO: handle when submission and post issues are separate (using referencing)
    pub async fn issue_comment_bounty_submission(
        &self,
        amount_requested: u128,
        bounty_id: u64,
        submission_id: u64,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .create_comment(
                issue_number,
                format!(
                    "${} Submission for BountyId {} with SubmissionId {}",
                    amount_requested, bounty_id, submission_id,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn issue_comment_submission_approval(
        &self,
        submission_id: u64,
        bounty_id: u64,
        transfer_amt: u128,
        remaining_balance: u128,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .create_comment(
                issue_number,
                format!(
                    "SubmissionId {} approved for BountyId {}, Transferring Balance ${} to the Submitter s.t. ${} Remaining Balance for the Bounty",
                    submission_id, bounty_id, transfer_amt, remaining_balance
                ),
            )
            .await?;
        Ok(())
    }
}
