use sunshine_bounty_gbot::{
    Error,
    GBot,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new()?;
    let last_bounty_comment = github_bot
        .get_last_comment(
            "sunshine-protocol".to_string(),
            "sunshine-bounty".to_string(),
            124,
        )
        .await?;
    println!("{:?}", last_bounty_comment.unwrap());
    // github_bot
    //     .new_bounty_issue(
    //         1738,
    //         1u64,
    //         "sunshine-protocol".to_string(),
    //         "sunshine-bounty".to_string(),
    //         124u64,
    //     )
    //     .await?;
    // println!("Bounty Post Succeeded");
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
