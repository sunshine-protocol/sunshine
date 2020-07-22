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
                    "${} Bounty Posted on Sunshine Chain With BountyId: {}",
                    amount, bounty_id,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn issue_comment_milestone_submission(
        &self,
        amount_requested: u128,
        bounty_id: u64,
        milestone_id: u64,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .create_comment(
                issue_number,
                format!(
                    "${} Milestone Submitted on Sunshine Chain with MilestoneId: ({}, {})",
                    amount_requested,
                    bounty_id,
                    milestone_id,
                ),
            )
            .await?;
        Ok(())
    }
}
