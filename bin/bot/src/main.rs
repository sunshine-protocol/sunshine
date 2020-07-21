mod command;
mod error;
mod subxt;
use crate::{
    command::*,
    error::{
        Error,
        Result,
    },
    subxt::*,
};
use clap::Clap;
use exitfailure::ExitDisplay;
use gbot::{
    BountyContext,
    GBot,
};
use ipfs_embed::Store;
use ipld_block_builder::ReadonlyCache;
use substrate_subxt::{
    sp_core::Decode,
    EventSubscription,
    EventsDecoder,
};
use sunshine_bounty_client::{
    bounty::{
        BountyEventsDecoder,
        BountyPostedEvent,
        MilestoneSubmittedEvent,
    },
    BountyBody,
};
use sunshine_core::ChainClient;
use sunshine_identity_cli::key::KeySetCommand;
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
async fn main() -> std::result::Result<(), ExitDisplay<Error>> {
    env_logger::init();
    let github_bot = GBot::new().map_err(Error::GithuBot)?;
    let opts: PathOpts = PathOpts::parse();
    let root = if let Some(root) = opts.path {
        root
    } else {
        dirs::config_dir().unwrap().join("sunshine-bounty-bot")
    };
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
    // CLI loop: subxt request -> update event subs -> gbot posts on github
    while let Ok(b) = run_cli(bot).await {
        bot = run_github_bot(b, github_bot.clone()).await?;
    }
    // TODO: report reason for exiting CLI (error in command input OR exit command invoked)
    Ok(())
}

async fn run_github_bot(mut bot: Bot, github: GBot) -> Result<Bot> {
    if let Some(Ok(raw)) = bot.bounty_post_sub.next().await {
        // get event data
        let event = BountyPostedEvent::<Runtime>::decode(&mut &raw.data[..])
            .map_err(Error::SubxtCodec)?;
        // fetch structured data from client
        let event_cid = event.description.to_cid().map_err(Error::CiDecode)?;
        let bounty_body = bot
            .client
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;
        // form the full bounty context from the body and the event amount
        let bounty_ctx =
            BountyContext::new(event.amount_reserved_for_bounty, bounty_body);
        github.post_bounty_in_issue(bounty_ctx).await?;
    } else if let Some(Ok(raw)) = bot.milestone_submit_sub.next().await {
        todo!();
    } else {
        time::delay_for(std::time::Duration::from_millis(100)).await;
    }
    Ok(bot)
}

async fn run_cli(mut bot: Bot) -> Result<Bot> {
    let opts: CommandOpts = CommandOpts::parse();
    match opts.cmd {
        SubCommand::Key(KeyCommand { cmd }) => {
            match cmd {
                KeySubCommand::Set(KeySetCommand {
                    force,
                    suri,
                    paperkey,
                }) => {
                    let account_id = sunshine_identity_cli::set_device_key(
                        &mut bot.client,
                        paperkey,
                        suri.as_deref(),
                        force,
                    )
                    .await?;
                    println!("your device key is {}", account_id.to_string());
                }
                KeySubCommand::Unlock(cmd) => cmd.exec(&mut bot.client).await?,
                KeySubCommand::Lock(cmd) => cmd.exec(&mut bot.client).await?,
            }
        }
        SubCommand::Org(OrgCommand { cmd }) => {
            match cmd {
                OrgSubCommand::IssueShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::BurnShares(cmd) => cmd.exec(&bot.client).await?,
                OrgSubCommand::BatchIssueShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::BatchBurnShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::ReserveShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::UnreserveShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::LockShares(cmd) => cmd.exec(&bot.client).await?,
                OrgSubCommand::UnlockShares(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::RegisterFlatOrg(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                OrgSubCommand::RegisterWeightedOrg(cmd) => {
                    cmd.exec(&bot.client).await?
                }
            }
        }
        SubCommand::Vote(VoteCommand { cmd }) => {
            match cmd {
                VoteSubCommand::CreateSignalThresholdVote(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                VoteSubCommand::CreatePercentThresholdVote(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                VoteSubCommand::CreateUnanimousConsentVote(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                VoteSubCommand::SubmitVote(cmd) => {
                    cmd.exec(&bot.client).await?
                }
            }
        }
        SubCommand::Bounty(BountyCommand { cmd }) => {
            match cmd {
                BountySubCommand::PostBounty(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::ApplyForBounty(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::TriggerApplicationReview(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::SudoApproveApplication(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::PollApplication(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::SubmitMilestone(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::TriggerMilestoneReview(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::SudoApproveMilestone(cmd) => {
                    cmd.exec(&bot.client).await?
                }
                BountySubCommand::PollMilestone(cmd) => {
                    cmd.exec(&bot.client).await?
                }
            }
        }
        // break out of while loop in main to exit the CLI
        SubCommand::Exit => return Err(Error::ExitBot),
    }
    Ok(bot)
}
