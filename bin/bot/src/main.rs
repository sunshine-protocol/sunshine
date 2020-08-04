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
        BountyPaymentExecutedEvent,
        BountyPostedEvent,
        BountyRaiseContributionEvent,
        BountySubmissionPostedEvent,
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
    pub bounty_contrib_sub: EventSubscription<Runtime>,
    pub bounty_submit_sub: EventSubscription<Runtime>,
    pub bounty_approval_sub: EventSubscription<Runtime>,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Error> {
    env_logger::init();
    let github_bot = GBot::new().map_err(Error::GithuBot)?;
    let root = dirs::config_dir().unwrap().join("sunshine-bounty-bot");
    let client = Client::new(&root, None).await.map_err(Error::Client)?;
    // subscribe to bounty posts
    let bounty_post_sub = bounty_post_subscriber(&client).await?;
    // subscribe to bounty contributions
    let bounty_contrib_sub = bounty_contribution_subscriber(&client).await?;
    // subscribe to bounty submissions
    let bounty_submit_sub = bounty_submission_subscriber(&client).await?;
    // subscribe to bounty payments
    let bounty_approval_sub = bounty_approval_subscriber(&client).await?;
    // instantiate local bot
    let mut bot = Bot {
        client,
        bounty_post_sub,
        bounty_contrib_sub,
        bounty_submit_sub,
        bounty_approval_sub,
    };
    // TODO: how can I read input so that the user doesn't have to press enter
    println!("Press `q` then `Enter` to quit the bounty bot");
    while keep_running_bot() {
        bot = run_github_bot(bot, github_bot.clone()).await?;
    }
    Ok(())
}

fn keep_running_bot() -> bool {
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        input.find('q').is_none()
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
                event.amount,
                event.id,
                bounty_body.repo_owner,
                bounty_body.repo_name,
                bounty_body.issue_number,
            )
            .await?;
    } else if let Some(Ok(raw)) = bot.bounty_contrib_sub.next().await {
        // get event data
        let event =
            BountyRaiseContributionEvent::<Runtime>::decode(&mut &raw.data[..])
                .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let event_cid = event.bounty_ref.to_cid().map_err(Error::CiDecode)?;
        let bounty_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;
        // issue comment
        github
            .issue_comment_bounty_contribute(
                event.amount,
                event.total,
                event.bounty_id,
                bounty_body.repo_owner,
                bounty_body.repo_name,
                bounty_body.issue_number,
            )
            .await?;
    } else if let Some(Ok(raw)) = bot.bounty_submit_sub.next().await {
        // get event data
        let event =
            BountySubmissionPostedEvent::<Runtime>::decode(&mut &raw.data[..])
                .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let bounty_event_cid =
            event.bounty_ref.to_cid().map_err(Error::CiDecode)?;
        let submission_event_cid =
            event.submission_ref.to_cid().map_err(Error::CiDecode)?;
        let bounty_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&bounty_event_cid)
            .await
            .map_err(Error::Libipld)?;
        let submission_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&submission_event_cid)
            .await
            .map_err(Error::Libipld)?;
        // issue comment
        github
            .issue_comment_bounty_submission(
                event.amount,
                event.bounty_id,
                event.id,
                submission_body.repo_owner,
                submission_body.repo_name,
                submission_body.issue_number,
                bounty_body.repo_owner,
                bounty_body.repo_name,
                bounty_body.issue_number,
            )
            .await?;
    } else if let Some(Ok(raw)) = bot.bounty_approval_sub.next().await {
        // get event data
        let event =
            BountyPaymentExecutedEvent::<Runtime>::decode(&mut &raw.data[..])
                .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let bounty_event_cid =
            event.bounty_ref.to_cid().map_err(Error::CiDecode)?;
        let submission_event_cid =
            event.submission_ref.to_cid().map_err(Error::CiDecode)?;
        let bounty_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&bounty_event_cid)
            .await
            .map_err(Error::Libipld)?;
        let submission_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&submission_event_cid)
            .await
            .map_err(Error::Libipld)?;
        // issue comment
        github
            .issue_comment_submission_approval(
                event.amount,
                event.new_total,
                event.submission_id,
                event.bounty_id,
                submission_body.repo_owner,
                submission_body.repo_name,
                submission_body.issue_number,
                bounty_body.repo_owner,
                bounty_body.repo_name,
                bounty_body.issue_number,
            )
            .await?;
    } else {
        time::delay_for(std::time::Duration::from_millis(100)).await;
    }
    Ok(bot)
}
