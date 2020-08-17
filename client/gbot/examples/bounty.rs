use sunshine_bounty_gbot::{
    Error,
    GBot,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new()?;
    github_bot
        .new_bounty_issue(
            1729,
            2u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            146u64,
        )
        .await?;
    println!("Bounty Post Succeeded");
    github_bot
        .update_bounty_issue(
            5000,
            2u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            146u64,
        )
        .await?;
    println!("Bounty Contribution Succeeded");
    github_bot
        .new_submission_issue(
            1000,
            2u64,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            146u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            111u64,
        )
        .await?;
    println!("Bounty Submission Succeeded");
    github_bot
        .approve_submission_issue(
            500,
            2u64,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            146u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            111u64,
        )
        .await?;
    println!("Bounty Approval Succeeded");
    Ok(())
}
