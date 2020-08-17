use sunshine_bounty_gbot::{
    Error,
    GBot,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    println!("Bot Started");
    let github_bot = GBot::new()?;
    println!("Authentication Succeeded");
    github_bot
        .new_bounty_issue(
            1738,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Post Succeeded");
    github_bot
        .update_bounty_issue(
            1748,
            1u64,
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124u64,
        )
        .await?;
    println!("Bounty Contribution Succeeded");
    Ok(())
}
