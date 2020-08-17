#![allow(clippy::too_many_arguments)]

mod error;
mod util;
pub use error::Error;
use error::Result;
use octocrab::{
    models::Comment,
    Octocrab,
};
use util::{
    GithubIssue,
    IssueComment,
};

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
    pub async fn get_last_comment(
        &self,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<Option<IssueComment>> {
        let page = self
            .crab
            .issues(repo_owner.clone(), repo_name.clone())
            .list_comments(issue_number)
            .since(chrono::Utc::now())
            .per_page(100)
            .page(2u32)
            .send()
            .await?;
        let mut comments_by_author = Vec::<Comment>::new();
        let current_user = self.crab.current().user().await?;
        // TODO: is this the right order? is there a better way to get the last comment
        for c in page {
            if c.user == current_user {
                comments_by_author.push(c);
            }
        }
        if let Some(last_comment) = comments_by_author.pop() {
            Ok(Some(IssueComment {
                issue: GithubIssue {
                    repo_name,
                    repo_owner,
                    issue_number,
                },
                comment: last_comment,
            }))
        } else {
            Ok(None)
        }
    }
}

#[macro_export]
macro_rules! fail {
    ( $y:expr ) => {{
        return Err($y.into())
    }};
}

#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            $crate::fail!($y);
        }
    }};
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
        ensure!(
            self.get_last_comment(
                repo_owner.clone(),
                repo_name.clone(),
                issue_number
            )
            .await?
            .is_none(),
            Error::CannotReuseIssues
        );
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
        ensure!(
            self.get_last_comment(
                submission_repo_owner.clone(),
                submission_repo_name.clone(),
                submission_issue_number
            )
            .await?
            .is_none(),
            Error::CannotReuseIssues
        );
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
