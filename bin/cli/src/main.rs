use crate::command::*;
use bounty_cli::{
    key::KeySetCommand,
    set_device_key,
    Command,
    Error,
};
use clap::Clap;
use exitfailure::ExitDisplay;
use ipfs_embed::{
    Config,
    Store,
};
use keybase_keystore::{
    DeviceKey,
    KeyStore,
    Password,
};
use sp_core::crypto::Pair;
use std::path::PathBuf;
use substrate_subxt::{
    sp_core,
    sp_core::sr25519,
    ClientBuilder,
};
use test_client::Runtime;

mod command;

#[async_std::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    Ok(run().await?)
}

type Client = test_client::bounty::Client<Runtime, sr25519::Pair, Store>;

struct Paths {
    _root: PathBuf,
    keystore: PathBuf,
    db: PathBuf,
}

impl Paths {
    fn new(root: Option<PathBuf>) -> Result<Self, Error> {
        let root = if let Some(root) = root {
            root
        } else {
            dirs2::config_dir()
                .ok_or(Error::ConfigDirNotFound)?
                .join("sunshine-cli")
        };
        let keystore = root.join("keystore");
        let db = root.join("db");
        Ok(Paths {
            _root: root,
            keystore,
            db,
        })
    }
}

async fn run() -> Result<(), Error> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    // initialize requisite storage utilities
    let paths = Paths::new(opts.path)?;
    let keystore = KeyStore::open(&paths.keystore).await?;
    // initialize keystore with alice's keys
    let alice_seed: [u8; 32] =
        sr25519::Pair::from_string_with_seed("//Alice", None)
            .unwrap()
            .1
            .unwrap();
    keystore
        .initialize(
            &DeviceKey::from_seed(alice_seed),
            &Password::from("password".to_string()),
        )
        .await?;
    let subxt = ClientBuilder::<Runtime>::new().build().await?;
    let config =
        Config::from_path(&paths.db).map_err(ipfs_embed::Error::Sled)?;
    let store = Store::new(config)?;
    // initialize new client from storage utilities
    let client = Client::new(keystore, subxt.clone(), store);
    // match on the passed in command
    match opts.cmd {
        SubCommand::Key(KeyCommand { cmd }) => {
            match cmd {
                KeySubCommand::Set(KeySetCommand {
                    force,
                    suri,
                    paperkey,
                }) => {
                    let account_id = set_device_key(
                        &client,
                        paperkey,
                        suri.as_deref(),
                        force,
                    )
                    .await?;
                    println!("your device key is {}", account_id.to_string());
                    Ok(())
                }
                KeySubCommand::Unlock(cmd) => cmd.exec(&client).await,
                KeySubCommand::Lock(cmd) => cmd.exec(&client).await,
            }
        }
        SubCommand::Wallet(WalletCommand { cmd }) => {
            match cmd {
                WalletSubCommand::GetAccountBalance(cmd) => {
                    cmd.exec(&client).await
                }
                WalletSubCommand::TransferBalance(cmd) => {
                    cmd.exec(&client).await
                }
            }
        }
        SubCommand::Org(OrgCommand { cmd }) => {
            match cmd {
                OrgSubCommand::IssueShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::BurnShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::BatchIssueShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::BatchBurnShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::ReserveShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::UnreserveShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::LockShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::UnlockShares(cmd) => cmd.exec(&client).await,
                OrgSubCommand::RegisterFlatOrg(cmd) => cmd.exec(&client).await,
                OrgSubCommand::RegisterWeightedOrg(cmd) => {
                    cmd.exec(&client).await
                }
            }
        }
        SubCommand::Run => {
            loop {
                async_std::task::sleep(std::time::Duration::from_millis(100))
                    .await
            }
        }
    }
}
