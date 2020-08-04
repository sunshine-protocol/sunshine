#![allow(clippy::too_many_arguments)]

mod error;
pub use error::Error;
use error::Result;

use octocrab::Octocrab;

const GITHUB_BASE_URL: &str = "https://github.com";

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
        last_contribution: u128,
        total_balance: u128,
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
                    "Contribution to Bounty {} of Balance ${} increases Total Bounty Balance to ${}",
                    bounty_id, last_contribution, total_balance
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn issue_comment_bounty_submission(
        &self,
        amount_requested: u128,
        bounty_id: u64,
        submission_id: u64,
        submission_repo_owner: String,
        submission_repo_name: String,
        submission_issue_number: u64,
        bounty_repo_owner: String,
        bounty_repo_name: String,
        bounty_issue_number: u64,
    ) -> Result<()> {
        let submission_issues_handler = self.crab.issues(
            submission_repo_owner.clone(),
            submission_repo_name.clone(),
        );
        let bounty_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            bounty_repo_owner,
            bounty_repo_name,
            bounty_issue_number
        );
        let _ = submission_issues_handler
            .create_comment(
                submission_issue_number,
                format!(
                    "${} Submission for BountyId {} with SubmissionId {} | [Bounty Reference]({})",
                    amount_requested, bounty_id, submission_id, bounty_issue_ref
                ),
            )
            .await?;
        let bounty_issues_handler =
            self.crab.issues(bounty_repo_owner, bounty_repo_name);
        let submission_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            submission_repo_owner,
            submission_repo_name,
            submission_issue_number
        );
        let _ = bounty_issues_handler
            .create_comment(
                bounty_issue_number,
                format!(
                    "${} Submission for Bounty with SubmissionId {} | [Submission Reference]({})",
                    amount_requested, submission_id, submission_issue_ref,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn issue_comment_submission_approval(
        &self,
        transfer_amt: u128,
        remaining_balance: u128,
        submission_id: u64,
        bounty_id: u64,
        submission_repo_owner: String,
        submission_repo_name: String,
        submission_issue_number: u64,
        bounty_repo_owner: String,
        bounty_repo_name: String,
        bounty_issue_number: u64,
    ) -> Result<()> {
        let submission_issues_handler = self.crab.issues(
            submission_repo_owner.clone(),
            submission_repo_name.clone(),
        );
        let bounty_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            bounty_repo_owner,
            bounty_repo_name,
            bounty_issue_number
        );
        let _ = submission_issues_handler
            .create_comment(
                submission_issue_number,
                format!(
                    "SubmissionId {} approved for BountyId {} and Transferred Balance ${} to the Submitter | [Bounty Reference]({})",
                    submission_id, bounty_id, transfer_amt, bounty_issue_ref,
                ),
            )
            .await?;
        let bounty_issues_handler =
            self.crab.issues(bounty_repo_owner, bounty_repo_name);
        let submission_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            submission_repo_owner,
            submission_repo_name,
            submission_issue_number
        );
        let _ = bounty_issues_handler
            .create_comment(
                bounty_issue_number,
                format!(
                    "${} Remaining Balance for the Bounty after SubmissionId {} approved for BountyId {}. Transferred Balance ${} to the Submitter | [Submission Reference]({})",
                    remaining_balance, submission_id, bounty_id, transfer_amt, submission_issue_ref,
                ),
            )
            .await?;
        Ok(())
    }
}
