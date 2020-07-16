mod bot;
mod error;
use error::Error;

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
use sp_keyring::AccountKeyring;
use std::path::PathBuf;
use substrate_subxt::{
    balances::{
        BalancesEventsDecoder,
        TransferCallExt,
        TransferEvent,
    },
    sp_core::{
        crypto::Pair,
        sr25519,
        Decode,
    },
    ClientBuilder,
    DefaultNodeRuntime,
    EventSubscription,
    EventsDecoder,
    PairSigner,
    Signer,
};
use test_client::{
    Client,
    Runtime,
};

#[tokio::main]
async fn main() -> Result<(), ExitDisplay<Error>> {
    Ok(run().await?)
}

#[derive(Clone, Debug, Clap)]
pub struct Opts {
    #[clap(short = "p", long = "path")]
    pub path: Option<PathBuf>,
    pub chain_spec_path: Option<PathBuf>,
}

async fn run() -> Result<(), Error> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    let root = if let Some(root) = opts.path {
        root
    } else {
        dirs::config_dir().unwrap().join("sunshine-bounty")
    };
    // initialize new client from storage utilities
    let client = Client::new(
        &root, None, // ChainSpec Config
    )
    .await?;
    // subscribe to bounty events

    // post bounty from client

    // get bounty event and bot post in issue if issue specified
    Ok(())
}
