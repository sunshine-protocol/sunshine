#![allow(clippy::too_many_arguments)]

mod error;
mod util;
use chrono::{
    DateTime,
    NaiveDate,
    NaiveDateTime,
    NaiveTime,
    TimeZone,
    Utc,
};
pub use error::Error;
use error::Result;
use octocrab::{
    models::issues::Comment,
    Octocrab,
};
use util::{
    Bounty,
    Submission,
};

const GITHUB_BASE_URL: &str = "https://github.com";

// TODO: const? lazy_static?
pub fn recent_time() -> DateTime<Utc> {
    let d = NaiveDate::from_ymd(2020, 08, 14);
    let t = NaiveTime::from_hms_milli(12, 34, 56, 789);
    let ndt = NaiveDateTime::new(d, t);
    DateTime::<Utc>::from_utc(ndt, Utc)
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
        let recent_time = recent_time();
        let mut page = self
            .crab
            .issues(repo_owner.clone(), repo_name.clone())
            .list_comments(issue_number)
            .since(recent_time)
            .send()
            .await?;
        let mut comments_by_author = Vec::<Comment>::new();
        let current_user = self.crab.current().user().await?;
        let mut items_on_page = page.take_items();
        Ok(items_on_page.pop())
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
    pub async fn new_bounty_issue(
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
        let bounty_comment = self
            .get_last_comment(
                repo_owner.clone(),
                repo_name.clone(),
                issue_number,
            )
            .await?
            .ok_or(Error::ContributionMustRefValidBountyIssue)?;
        // let posted_bounty = bounty_comment
        //     .body
        //     .ok_or(Error::ContributionMustRefValidBountyIssue)?
        //     .parse::<Bounty>()?;
        // ensure!(
        //     posted_bounty.id == bounty_id,
        //     Error::CannotUpdateDifferentBounty
        // );
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
}
