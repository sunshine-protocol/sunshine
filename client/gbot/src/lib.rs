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
    pub fn new() -> Result<Self> {
        let crab = Octocrab::builder()
            .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
            .build()?;
        Ok(GBot { crab })
    }
}

pub struct BountyContext {
    pub amount: u128,
    pub body: BountyBody,
}

impl BountyContext {
    pub fn new(amount: u128, body: BountyBody) -> BountyContext {
        BountyContext { amount, body }
    }
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
