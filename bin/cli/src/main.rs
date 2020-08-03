use crate::command::*;
use clap::Clap;
use exitfailure::ExitDisplay;
use substrate_subxt::{
    balances::*,
    system::*,
};
use sunshine_core::{
    ChainClient,
    Ss58,
};
use sunshine_identity_cli::{
    key::KeySetCommand,
    wallet::{
        WalletBalanceCommand,
        WalletTransferCommand,
    },
};
use test_client::{
    Client,
    Runtime,
};
use thiserror::Error;

mod command;

#[async_std::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    Ok(run().await?)
}

#[derive(Debug, Error)]
enum Error {
    #[error(transparent)]
    Bounty(#[from] sunshine_bounty_cli::Error<test_client::Error>),
    #[error(transparent)]
    Identity(#[from] sunshine_identity_cli::Error<test_client::Error>),
    #[error(transparent)]
    Client(#[from] test_client::Error),
    #[error(transparent)]
    Ss58(#[from] sunshine_core::InvalidSs58),
}

async fn run() -> Result<(), Error> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    let root = if let Some(root) = opts.path {
        root
    } else {
        dirs::config_dir().unwrap().join("sunshine-bounty")
    };
    let mut client = Client::new(&root, None).await?;

    match opts.cmd {
        SubCommand::Key(KeyCommand { cmd }) => {
            match cmd {
                KeySubCommand::Set(KeySetCommand {
                    force,
                    suri,
                    paperkey,
                }) => {
                    let account_id = sunshine_identity_cli::set_device_key(
                        &mut client,
                        paperkey,
                        suri.as_deref(),
                        force,
                    )
                    .await?;
                    println!("your device key is {}", account_id.to_string());
                }
                KeySubCommand::Unlock(cmd) => cmd.exec(&mut client).await?,
                KeySubCommand::Lock(cmd) => cmd.exec(&mut client).await?,
            }
        }
        SubCommand::Wallet(WalletCommand { cmd }) => {
            match cmd {
                WalletSubCommand::GetAccountBalance(WalletBalanceCommand {
                    identifier,
                }) => {
                    let account_id: Ss58<Runtime> =
                        if let Some(identifier) = identifier {
                            identifier.parse()?
                        } else {
                            Ss58(
                                client
                                    .chain_signer()
                                    .map_err(Error::Client)?
                                    .account_id()
                                    .clone(),
                            )
                        };
                    let account = client
                        .chain_client()
                        .account(&account_id.0, None)
                        .await
                        .map_err(|e| Error::Client(e.into()))?;
                    println!("{}", account.data.free);
                }
                WalletSubCommand::TransferBalance(WalletTransferCommand {
                    identifier,
                    amount,
                }) => {
                    let account_id: Ss58<Runtime> = identifier.parse()?;
                    let signer =
                        client.chain_signer().map_err(Error::Client)?;
                    let event = client
                        .chain_client()
                        .transfer_and_watch(signer, &account_id.0, amount)
                        .await
                        .map_err(|e| Error::Client(e.into()))?
                        .transfer()
                        .map_err(|e| Error::Client(e.into()))?
                        .unwrap();
                    println!(
                        "transfered {} to {}",
                        event.amount,
                        event.to.to_string()
                    );
                }
            }
        }
        SubCommand::Org(OrgCommand { cmd }) => {
            match cmd {
                OrgSubCommand::IssueShares(cmd) => cmd.exec(&client).await?,
                OrgSubCommand::BurnShares(cmd) => cmd.exec(&client).await?,
                OrgSubCommand::BatchIssueShares(cmd) => {
                    cmd.exec(&client).await?
                }
                OrgSubCommand::BatchBurnShares(cmd) => {
                    cmd.exec(&client).await?
                }
                OrgSubCommand::ReserveShares(cmd) => cmd.exec(&client).await?,
                OrgSubCommand::UnreserveShares(cmd) => {
                    cmd.exec(&client).await?
                }
                OrgSubCommand::LockShares(cmd) => cmd.exec(&client).await?,
                OrgSubCommand::UnlockShares(cmd) => cmd.exec(&client).await?,
                OrgSubCommand::RegisterFlatOrg(cmd) => {
                    cmd.exec(&client).await?
                }
                OrgSubCommand::RegisterWeightedOrg(cmd) => {
                    cmd.exec(&client).await?
                }
            }
        }
        SubCommand::Vote(VoteCommand { cmd }) => {
            match cmd {
                VoteSubCommand::CreateSignalThresholdVote(cmd) => {
                    cmd.exec(&client).await?
                }
                VoteSubCommand::CreatePercentThresholdVote(cmd) => {
                    cmd.exec(&client).await?
                }
                VoteSubCommand::CreateUnanimousConsentVote(cmd) => {
                    cmd.exec(&client).await?
                }
                VoteSubCommand::SubmitVote(cmd) => cmd.exec(&client).await?,
            }
        }
        SubCommand::Donate(DonateCommand { cmd }) => {
            match cmd {
                DonateSubCommand::PropDonate(cmd) => cmd.exec(&client).await?,
                DonateSubCommand::EqualDonate(cmd) => cmd.exec(&client).await?,
            }
        }
        SubCommand::Bank(BankCommand { cmd }) => {
            match cmd {
                BankSubCommand::OpenAccount(cmd) => cmd.exec(&client).await?,
                BankSubCommand::OpenAccount2(cmd) => cmd.exec(&client).await?,
            }
        }
        SubCommand::Bounty(BountyCommand { cmd }) => {
            match cmd {
                BountySubCommand::PostBounty(cmd) => cmd.exec(&client).await?,
                BountySubCommand::ContributeToBounty(cmd) => {
                    cmd.exec(&client).await?
                }
                BountySubCommand::SubmitForBounty(cmd) => {
                    cmd.exec(&client).await?
                }
                BountySubCommand::ApproveApplication(cmd) => {
                    cmd.exec(&client).await?
                }
                BountySubCommand::GetBounty(cmd) => cmd.exec(&client).await?,
                BountySubCommand::GetSubmission(cmd) => {
                    cmd.exec(&client).await?
                }
                BountySubCommand::GetOpenBounties(cmd) => {
                    cmd.exec(&client).await?
                }
                BountySubCommand::GetOpenSubmissions(cmd) => {
                    cmd.exec(&client).await?
                }
            }
        }
    }
    Ok(())
}
