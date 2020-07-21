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
use ipfs_embed::Store;
use substrate_subxt::EventSubscription;
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
    let opts: Opts = Opts::parse();
    let root = if let Some(root) = opts.path {
        root
    } else {
        dirs::config_dir().unwrap().join("sunshine-bounty-bot")
    };
    let client = Client::new(&root, None).await.map_err(Error::Client)?;
    // subscribe to bounty posts
    let bounty_post_sub = bounty_post_subscriber(&client).await?;
    let milestone_submit_sub = milestone_submission_subscriber(&client).await?;
    let mut bot = Bot {
        client,
        bounty_post_sub,
        milestone_submit_sub,
    };
    // TODO: add back password_changes_sub here and add to Bot to update client
    while let Ok(mut b) = run_cli(bot).await {
        if let Some(sub) = b.bounty_post_sub.next().await {
            // bot gets event data
            // bot fetches cid from offchain_client
            // bot posts comment with new `BountyContext`
            todo!();
        } else if let Some(sub) = b.milestone_submit_sub.next().await {
            // bot gets event data
            // bot fetches cid from offchain_client
            // bot posts comment with new `BountyContext`
            todo!();
        } else {
            time::delay_for(std::time::Duration::from_millis(100)).await;
        }
        bot = b;
    }
    Ok(())
}

async fn run_cli(mut bot: Bot) -> Result<Bot> {
    let opts: Opts = Opts::parse();
    // run the cli
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
