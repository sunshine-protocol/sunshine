mod subxt;
use crate::subxt::*;
use gbot::GBot;
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
use sunshine_client_utils::{
    Client as _,
    Result,
};
use test_client::{
    Client,
    Runtime,
};
use tokio::time;

pub struct Bot {
    pub client: Client,
    pub bounty_post_sub: EventSubscription<Runtime>,
    pub bounty_contrib_sub: EventSubscription<Runtime>,
    pub bounty_submit_sub: EventSubscription<Runtime>,
    pub bounty_approval_sub: EventSubscription<Runtime>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let github_bot = GBot::new()?;
    let root = dirs::config_dir().unwrap().join("sunshine-bounty-bot");
    let client = Client::new(&root, None).await?;
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
    loop {
        bot = run_github_bot(bot, github_bot.clone()).await?;
    }
}

async fn run_github_bot(mut bot: Bot, github: GBot) -> Result<Bot> {
    if let Some(Ok(raw)) = bot.bounty_post_sub.next().await {
        // get event data
        let event = BountyPostedEvent::<Runtime>::decode(&mut &raw.data[..])?;
        // fetch structured data from client
        let event_cid = event.description.to_cid()?;
        let bounty_body: BountyBody =
            bot.client.offchain_client().get(&event_cid).await?;
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
        let event = BountyRaiseContributionEvent::<Runtime>::decode(
            &mut &raw.data[..],
        )?;
        // fetch structured data from client
        let event_cid = event.bounty_ref.to_cid()?;
        let bounty_body: BountyBody =
            bot.client.offchain_client().get(&event_cid).await?;
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
            BountySubmissionPostedEvent::<Runtime>::decode(&mut &raw.data[..])?;
        // fetch structured data from client
        let bounty_event_cid = event.bounty_ref.to_cid()?;
        let submission_event_cid = event.submission_ref.to_cid()?;
        let bounty_body: BountyBody =
            bot.client.offchain_client().get(&bounty_event_cid).await?;
        let submission_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&submission_event_cid)
            .await?;
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
            BountyPaymentExecutedEvent::<Runtime>::decode(&mut &raw.data[..])?;
        // fetch structured data from client
        let bounty_event_cid = event.bounty_ref.to_cid()?;
        let submission_event_cid = event.submission_ref.to_cid()?;
        let bounty_body: BountyBody =
            bot.client.offchain_client().get(&bounty_event_cid).await?;
        let submission_body: BountyBody = bot
            .client
            .offchain_client()
            .get(&submission_event_cid)
            .await?;
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
