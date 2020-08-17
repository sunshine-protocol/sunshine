#![allow(clippy::too_many_arguments)]

mod error;
mod parser;
use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    Utc,
};
pub use error::Error;
use error::Result;
use octocrab::{
    models::issues::Comment,
    Octocrab,
};

const GITHUB_BASE_URL: &str = "https://github.com";

fn recent_time() -> DateTime<Utc> {
    let d = NaiveDate::from_ymd(2020, 8, 14);
    let t = NaiveTime::from_hms_milli(12, 34, 56, 789);
    let ndt = NaiveDateTime::new(d, t);
    DateTime::<Utc>::from_utc(ndt, Utc)
}

#[macro_use]
extern crate lazy_static;
lazy_static! {
    static ref RECENT_TIME: DateTime<Utc> = recent_time();
}

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
    ) -> Result<Option<Comment>> {
        let page = self
            .crab
            .issues(repo_owner.clone(), repo_name.clone())
            .list_comments(issue_number)
            .since(*RECENT_TIME)
            .send()
            .await?;
        let current_user = self.crab.current().user().await?;
        for c in page.into_iter().rev() {
            if c.user == current_user {
                return Ok(Some(c))
            }
        }
        Ok(None)
    }
}

#[macro_export]
macro_rules! ensure {
    ( $x:expr, $y:expr $(,)? ) => {{
        if !$x {
            return Err($y.into())
        }
    }};
}

impl GBot {
    pub async fn new_bounty_issue(
        &self,
        amount: u128,
        bounty_id: u64,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        // TODO: move check to before chain client call
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
                    "☀️ Sunshine Bounty Posted ☀️ \n
                    BountyID: {} | Total Amount: {}",
                    bounty_id, amount,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn update_bounty_issue(
        &self,
        new_balance: u128,
        bounty_id: u64,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
    ) -> Result<()> {
        // TODO: move check to before chain client call
        let bounty_comment = self
            .get_last_comment(
                repo_owner.clone(),
                repo_name.clone(),
                issue_number,
            )
            .await?
            .ok_or(Error::MustRefValidBountyIssue)?;
        // TODO: parse comment into Bounty and verify that bounty_id are equal
        let new_issues_handler = self.crab.issues(repo_owner, repo_name);
        let _ = new_issues_handler
            .update_comment(
                bounty_comment.id,
                format!(
                    "☀️ Sunshine Bounty Posted ☀️ \n
                    BountyID: {} | Total Amount: {}",
                    bounty_id, new_balance,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn new_submission_issue(
        &self,
        amount: u128,
        bounty_id: u64,
        submission_id: u64,
        bounty_repo_owner: String,
        bounty_repo_name: String,
        bounty_issue_number: u64,
        submission_repo_owner: String,
        submission_repo_name: String,
        submission_issue_number: u64,
    ) -> Result<()> {
        // TODO: move check to before chain client call
        ensure!(
            self.get_last_comment(
                bounty_repo_owner.clone(),
                bounty_repo_name.clone(),
                bounty_issue_number
            )
            .await?
            .is_some(),
            Error::MustRefValidBountyIssue
        ); // TODO: check that the issue refers to the same BountyID
        let bounty_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            bounty_repo_owner,
            bounty_repo_name,
            bounty_issue_number
        );
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
        let new_issues_handler = self
            .crab
            .issues(submission_repo_owner, submission_repo_name);
        let _ = new_issues_handler
            .create_comment(
                submission_issue_number,
                format!(
                    "☀️ Sunshine Submission Posted ☀️ \n
                    BountyID: {} | SubmissionID: {} | Amount Requested: {} | [Bounty Issue]({})",
                    bounty_id, submission_id, amount, bounty_issue_ref,
                ),
            )
            .await?;
        Ok(())
    }
    pub async fn approve_submission_issue(
        &self,
        amount_received: u128,
        bounty_id: u64,
        submission_id: u64,
        bounty_repo_owner: String,
        bounty_repo_name: String,
        bounty_issue_number: u64,
        submission_repo_owner: String,
        submission_repo_name: String,
        submission_issue_number: u64,
    ) -> Result<()> {
        let bounty_issue_ref = format!(
            "{}/{}/{}/issues/{}",
            GITHUB_BASE_URL,
            bounty_repo_owner,
            bounty_repo_name,
            bounty_issue_number
        );
        // TODO: move check to before chain client call
        let submission_comment = self
            .get_last_comment(
                submission_repo_owner.clone(),
                submission_repo_name.clone(),
                submission_issue_number,
            )
            .await?
            .ok_or(Error::MustRefValidSubmissionIssue)?;
        // TODO: parse comment into Submission and verify bounty_id, submission_id are valid
        let new_issues_handler = self
            .crab
            .issues(submission_repo_owner, submission_repo_name);
        let _ = new_issues_handler
            .update_comment(
                submission_comment.id,
                format!(
                    "☀️ Sunshine Submission Approved ☀️ \n
                    BountyID: {} | SubmissionID: {} | Amount Received: {} | [Bounty Issue]({})",
                    bounty_id, submission_id, amount_received, bounty_issue_ref,
                ),
            )
            .await?;
        Ok(())
    }
}
