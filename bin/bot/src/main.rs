mod error;
mod subxt;
use crate::{
    error::{
        Error,
        Result,
    },
    subxt::*,
};
use gbot::GBot;
use ipfs_embed::Store;
use ipld_block_builder::ReadonlyCache;
use substrate_subxt::{
    sp_core::Decode,
    EventSubscription,
};
use sunshine_bounty_client::{
    bounty::{
        BountyPostedEvent,
        MilestoneSubmittedEvent,
    },
    BountyBody,
};
use sunshine_core::ChainClient;
use test_client::{
    Client,
    Runtime,
};
use tokio::time;

pub struct Bot {
    pub client: Client<Store>,
    pub bounty_post_sub: EventSubscription<Runtime>,
    pub milestone_submit_sub: EventSubscription<Runtime>,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new().map_err(Error::GithuBot)?;
    let root = dirs::config_dir().unwrap().join("sunshine-bounty-bot");
    let client = Client::new(&root, None).await.map_err(Error::Client)?;
    // subscribe to bounty posts
    let bounty_post_sub = bounty_post_subscriber(&client).await?;
    // subscribe to milestone submissions
    let milestone_submit_sub = milestone_submission_subscriber(&client).await?;
    // instantiate local bot
    let mut bot = Bot {
        client,
        bounty_post_sub,
        milestone_submit_sub,
    };
    // String buffer for possible std input
    println!("Press `q` to quit the bounty bot");
    while keep_running_bot() {
        bot = run_github_bot(bot, github_bot.clone()).await?;
    }
    Ok(())
}

fn keep_running_bot() -> bool {
    let mut input = String::new();
    if let Ok(_) = std::io::stdin().read_line(&mut input) {
        if let Some(_) = input.find("q") {
            false
        } else {
            true
        }
    } else {
        true
    }
}

async fn run_github_bot(mut bot: Bot, github: GBot) -> Result<Bot> {
    if let Some(Ok(raw)) = bot.bounty_post_sub.next().await {
        // get event data
        let event = BountyPostedEvent::<Runtime>::decode(&mut &raw.data[..])
            .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let event_cid = event.description.to_cid().map_err(Error::CiDecode)?;
        let bounty_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;
        // issue comment
        github
            .issue_comment_bounty_post(
                event.amount_reserved_for_bounty,
                event.new_bounty_id,
                bounty_body.repo_owner,
                bounty_body.repo_name,
                bounty_body.issue_number,
            )
            .await?;
    } else if let Some(Ok(raw)) = bot.milestone_submit_sub.next().await {
        // get event data
        let event =
            MilestoneSubmittedEvent::<Runtime>::decode(&mut &raw.data[..])
                .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let event_cid =
            event.submission_ref.to_cid().map_err(Error::CiDecode)?;
        let milestone_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;
        // issue comment
        github
            .issue_comment_milestone_submission(
                event.amount_requested,
                event.bounty_id,
                event.new_milestone_id,
                milestone_body.repo_owner,
                milestone_body.repo_name,
                milestone_body.issue_number,
            )
            .await?;
    } else {
        time::delay_for(std::time::Duration::from_millis(100)).await;
    }
    Ok(bot)
}
