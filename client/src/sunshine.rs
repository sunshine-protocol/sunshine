use crate::error::Error;
#[cfg(feature = "light-client")]
use crate::light_client::ChainType;
use crate::runtime::{Client, ClientBuilder, Pair, PairSigner, Runtime};
use crate::srml::org::*;
use ipfs_embed::{Config, Store};
use ipld_block_builder::{BlockBuilder, Codec};
use sp_core::Pair as _;
use std::path::Path;
use substrate_subxt::system::*;

pub struct SunClient {
    client: Client,
    ipld: BlockBuilder<Store, Codec>,
}

impl SunClient {
    pub async fn new<P: AsRef<Path>>(path: P, signer: Pair) -> Result<Self, Error> {
        let db = sled::open(path)?;
        let ipld_tree = db.open_tree("ipld_tree")?;
        let config = Config::from_tree(ipld_tree);
        let store = Store::new(config)?;
        let codec = Codec::new();
        let ipld = BlockBuilder::new(store, codec);
        let client = crate::runtime::ClientBuilder::new().build().await?;
        Ok(Self { client, ipld })
    }
    /// Returns a signer for alice
    pub fn alice_signer(&self) -> PairSigner {
        PairSigner::new(sp_keyring::sr25519::Keyring::Alice.pair())
    }
    /// Reserves shares for alice
    pub async fn reserve_shares(&self, org: u64) -> Result<SharesReservedEvent<Runtime>, Error> {
        self.client
            .clone()
            .reserve_shares_and_watch(
                &self.alice_signer(),
                org,
                &sp_keyring::sr25519::Keyring::Alice.to_account_id(),
            )
            .await?
            .shares_reserved()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
}
