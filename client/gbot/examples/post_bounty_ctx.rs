use sunshine_bounty_gbot::{
    Error,
    GBot,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new()?;
    // make the call to post in the given issue
    github_bot
        .issue_comment_bounty_post(
            4444u128,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Post Succeeded");
    // make the call to post in the given issue
    github_bot
        .issue_comment_bounty_submission(
            333u128,
            1u64,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Submission Succeeded");
    Ok(())
}
