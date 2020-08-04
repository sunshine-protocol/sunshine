use sunshine_bounty_gbot::{
    Error,
    GBot,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new()?;
    github_bot
        .issue_comment_bounty_post(
            1738,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Post Succeeded");
    github_bot
        .issue_comment_bounty_contribute(
            10,
            1748,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Contribution Post Succeeded");
    github_bot
        .issue_comment_bounty_submission(
            100u128,
            1u64,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            141u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Post Submission Succeeded");
    github_bot
        .issue_comment_submission_approval(
            100u128,
            1648u128,
            1u64,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            141u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Submission Approval Succeeded");
    Ok(())
}
