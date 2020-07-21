mod error;
pub use error::Error;
use error::Result;

use octocrab::Octocrab;
use sunshine_bounty_client::BountyBody;

#[derive(Debug, Clone)]
pub struct GBot {
    crab: Octocrab,
}

impl GBot {
    pub fn new() -> Result<Octocrab> {
        let new_crab = Octocrab::builder()
            .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
            .build()?;
        Ok(new_crab)
    }
}

// TODO: Parse from substrate-subxt event
pub struct BountyContext {
    pub amount: u64,
    pub body: BountyBody,
}

impl GBot {
    pub async fn post_bounty_in_issue(&self, ctx: BountyContext) -> Result<()> {
        let new_issues_handler =
            self.crab.issues(ctx.body.repo_owner, ctx.body.repo_name);
        let _ = new_issues_handler
            .create_comment(
                ctx.body.issue_number,
                format!(
                    "Bounty Amount {} Posted on Sunshine Chain For This Issue",
                    ctx.amount
                ),
            )
            .await?;
        Ok(())
    }
}
