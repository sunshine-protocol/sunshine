use crate::error::Result;
use octocrab::Octocrab;

#[derive(Debug, Clone)]
pub struct Bot {
    crab: Octocrab,
}

impl Bot {
    pub fn new() -> Result<Octocrab> {
        let new_crab = Octocrab::builder()
            .personal_token(std::env::var("GITHUB_TOKEN").unwrap())
            .build()?;
        Ok(new_crab)
    }
}

// TODO: impl From for PostedBountyEventExt
pub struct BountyContext {
    pub amount: u64,
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
}

impl Bot {
    pub async fn post_bounty_in_issue(&self, ctx: BountyContext) -> Result<()> {
        let new_issues_handler =
            self.crab.issues(ctx.repo_owner, ctx.repo_name);
        let new_comment = new_issues_handler
            .create_comment(
                ctx.issue_number,
                format!(
                    "Bounty Amount {} Posted on Sunshine Chain For This Issue",
                    ctx.amount
                ),
            )
            .await?;
        Ok(())
    }
}
