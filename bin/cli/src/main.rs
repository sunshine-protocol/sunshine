use crate::command::*;
use crate::error::Error;
use crate::runtime::{AccountId, Extra, IpfsReference, OrgId, Runtime, Shares, Signature};
use clap::Clap;
use core::convert::TryInto;
use exitfailure::ExitDisplay;
use ipfs_embed::{Config, Store};
use ipld_block_builder::{BlockBuilder, Codec};
use keybase_keystore::bip39::{Language, Mnemonic};
use keybase_keystore::{DeviceKey, KeyStore, Password};
use std::path::PathBuf;
use substrate_subxt::balances::{TransferCallExt, TransferEventExt};
use substrate_subxt::sp_core::{crypto::Ss58Codec, sr25519};
use substrate_subxt::{ClientBuilder, Signer};
use textwrap::Wrapper;

mod command;
mod error;
mod runtime;

#[async_std::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    Ok(run().await?)
}

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
            dirs::config_dir()
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

type Client = sunshine_client::SunClient<Runtime, Signature, Extra, sr25519::Pair, Store>;

async fn run() -> Result<(), Error> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    let paths = Paths::new(opts.path)?;

    let keystore = KeyStore::new(&paths.keystore);
    let subxt = ClientBuilder::<Runtime>::new().build().await?;
    let config = Config::from_path(&paths.db).map_err(ipfs_embed::Error::Sled)?;
    let store = Store::new(config)?;
    let codec = Codec::new();
    let ipld = BlockBuilder::new(store, codec);

    let mut client = Client::new(keystore, subxt.clone(), ipld);

    match opts.cmd {
        SubCommand::Key(KeyCommand { cmd }) => match cmd {
            KeySubCommand::Set(KeySetCommand {
                force,
                suri,
                paperkey,
            }) => {
                // TODO: initialize key
                println!("lock and unlock left unimplemented");
            }
            _ => {
                println!("lock and unlock left unimplemented");
            }
            // KeySubCommand::Unlock => {
            //     let password = ask_for_password("Please enter your password (8+ characters):\n")?;
            //     client.unlock(&password)?;
            // }
            // KeySubCommand::Lock => client.lock()?,
        },
        SubCommand::Wallet(WalletCommand { cmd }) => match cmd {
            WalletSubCommand::IssueShares(WalletIssueSharesCommand {
                organization,
                who,
                shares,
            }) => {
                let org: OrgId = organization.try_into()?;
                let recipient: AccountId = who.into_account().ok_or(Error::UnparsedIdentifier)?; // TODO custom error
                let new_shares_minted: Shares = shares.try_into()?;
                let _ = client
                    .issue_shares(org, recipient, new_shares_minted)
                    .await?;
                // TODO: print event emittance like this
                // println!("{} of free balance", balance);
            }
            WalletSubCommand::ReserveShares(WalletReserveSharesCommand { organization, who }) => {
                let org: OrgId = organization.try_into()?;
                let owner_of_reserved_shares: AccountId =
                    who.into_account().ok_or(Error::UnparsedIdentifier)?; // TODO custom error
                let _ = client
                    .reserve_shares(org, &owner_of_reserved_shares)
                    .await?;
                // TODO: print event emittance like this
                // println!("{} of free balance", balance);
            }
            WalletSubCommand::LockShares(WalletLockSharesCommand { organization, who }) => {
                let org: OrgId = organization.try_into()?;
                let owner_of_locked_shares: AccountId =
                    who.into_account().ok_or(Error::UnparsedIdentifier)?; // TODO custom error
                let _ = client.lock_shares(org, &owner_of_locked_shares).await?;
                // TODO: print event emittance like this
                // println!("{} of free balance", balance);
            }
        },
        SubCommand::Run => loop {
            async_std::task::sleep(std::time::Duration::from_millis(100)).await
        },
    }
    Ok(())
}

fn ask_for_password(prompt: &str) -> Result<Password, Error> {
    Ok(Password::from(rpassword::prompt_password_stdout(prompt)?))
}

async fn ask_for_phrase(prompt: &str) -> Result<Mnemonic, Error> {
    println!("{}", prompt);
    let mut words = Vec::with_capacity(24);
    while words.len() < 24 {
        let mut line = String::new();
        async_std::io::stdin().read_line(&mut line).await?;
        for word in line.split(' ') {
            words.push(word.trim().to_string());
        }
    }
    println!("");
    Ok(Mnemonic::from_phrase(&words.join(" "), Language::English)
        .map_err(|_| Error::InvalidMnemonic)?)
}

// async fn resolve(client: &mut Client, identifier: Option<Identifier>) -> Result<Uid, Error> {
//     let identifier = if let Some(identifier) = identifier {
//         identifier
//     } else {
//         Identifier::Account(client.signer()?.account_id().clone())
//     };
//     let uid = match identifier {
//         Identifier::Uid(uid) => uid,
//         Identifier::Account(account_id) => client
//             .fetch_uid(&account_id)
//             .await?
//             .ok_or(Error::NoAccount)?,
//         Identifier::
//     };
//     Ok(uid)
// }
