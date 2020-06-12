use crate::error::Error;
#[cfg(feature = "light-client")]
use crate::light_client::ChainType;
use crate::runtime::{Client, PairSigner, Runtime};
use crate::srml::org::*;
use ipfs_embed::{Config, Store};
use ipld_block_builder::{BlockBuilder, Codec};
// use sp_core::Pair as _;
use std::path::Path;
use substrate_subxt::{sp_runtime::AccountId32, system::*};
use utils_identity::cid::CidBytes;

pub struct SunClient {
    client: Client,
    ipld: BlockBuilder<Store, Codec>,
}

impl SunClient {
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
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
    /// Register flat organization
    pub async fn register_flat_org(
        &self,
        sudo: Option<AccountId32>,
        parent_org: Option<u64>,
        constitution: CidBytes,
        members: &[AccountId32],
    ) -> Result<FlatOrgRegisteredEvent<Runtime>, Error> {
        self.client
            .clone()
            .register_flat_org_and_watch(
                &self.alice_signer(),
                sudo,
                parent_org,
                constitution,
                members,
            )
            .await?
            .flat_org_registered()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Register weighted organization
    pub async fn register_weighted_org(
        &self,
        sudo: Option<AccountId32>,
        parent_org: Option<u64>,
        constitution: CidBytes,
        weighted_members: &[(AccountId32, u64)],
    ) -> Result<WeightedOrgRegisteredEvent<Runtime>, Error> {
        self.client
            .clone()
            .register_weighted_org_and_watch(
                &self.alice_signer(),
                sudo,
                parent_org,
                constitution,
                weighted_members,
            )
            .await?
            .weighted_org_registered()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Issue shares
    pub async fn issue_shares(
        &self,
        organization: u64,
        who: AccountId32,
        shares: u64,
    ) -> Result<SharesIssuedEvent<Runtime>, Error> {
        self.client
            .clone()
            .issue_shares_and_watch(&self.alice_signer(), organization, &who, shares)
            .await?
            .shares_issued()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Burn shares
    pub async fn burn_shares(
        &self,
        organization: u64,
        who: AccountId32,
        shares: u64,
    ) -> Result<SharesBurnedEvent<Runtime>, Error> {
        self.client
            .clone()
            .burn_shares_and_watch(&self.alice_signer(), organization, &who, shares)
            .await?
            .shares_burned()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Batch issue shares
    pub async fn batch_issue_shares(
        &self,
        organization: u64,
        new_accounts: &[(AccountId32, u64)],
    ) -> Result<SharesBatchIssuedEvent<Runtime>, Error> {
        self.client
            .clone()
            .batch_issue_shares_and_watch(&self.alice_signer(), organization, new_accounts)
            .await?
            .shares_batch_issued()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Batch burn shares
    pub async fn batch_burn_shares(
        &self,
        organization: u64,
        old_accounts: &[(AccountId32, u64)],
    ) -> Result<SharesBatchBurnedEvent<Runtime>, Error> {
        self.client
            .clone()
            .batch_burn_shares_and_watch(&self.alice_signer(), organization, old_accounts)
            .await?
            .shares_batch_burned()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
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
    /// Reserves shares for alice
    pub async fn unreserve_shares(
        &self,
        org: u64,
    ) -> Result<SharesUnReservedEvent<Runtime>, Error> {
        self.client
            .clone()
            .unreserve_shares_and_watch(
                &self.alice_signer(),
                org,
                &sp_keyring::sr25519::Keyring::Alice.to_account_id(),
            )
            .await?
            .shares_un_reserved()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Lock shares for alice
    pub async fn lock_shares(&self, org: u64) -> Result<SharesLockedEvent<Runtime>, Error> {
        self.client
            .clone()
            .lock_shares_and_watch(
                &self.alice_signer(),
                org,
                &sp_keyring::sr25519::Keyring::Alice.to_account_id(),
            )
            .await?
            .shares_locked()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Unlock shares for alice
    pub async fn unlock_shares(&self, org: u64) -> Result<SharesUnlockedEvent<Runtime>, Error> {
        self.client
            .clone()
            .unlock_shares_and_watch(
                &self.alice_signer(),
                org,
                &sp_keyring::sr25519::Keyring::Alice.to_account_id(),
            )
            .await?
            .shares_unlocked()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
}
