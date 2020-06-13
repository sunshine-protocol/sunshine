use crate::error::{Error, Result};
#[cfg(feature = "light-client")]
use crate::light_client::ChainType;
use crate::runtime::{Client, Pair, PairSigner, Runtime};
use crate::srml::org::*;
use ipfs_embed::Store;
use ipld_block_builder::{BlockBuilder, Codec};
use keystore::{DeviceKey, KeyStore, Password};
use std::path::Path;
use substrate_subxt::sp_core::crypto::Pair as SubPair;
use substrate_subxt::{sp_runtime::AccountId32, Signer};
use utils_identity::cid::CidBytes;

#[derive(new)]
pub struct SunClient {
    client: Client,
    keystore: KeyStore,
    ipld: BlockBuilder<Store, Codec>,
}

impl SunClient {
    /// Set device key, directly from substrate-identity to use with keystore
    pub fn has_device_key(&self) -> bool {
        self.keystore.is_initialized()
    }
    /// Set device key, directly from substrate-identity to use with keystore
    pub fn set_device_key(
        &self,
        dk: &DeviceKey,
        password: &Password,
        force: bool,
    ) -> Result<AccountId32> {
        if self.keystore.is_initialized() && !force {
            return Err(Error::KeystoreInitialized);
        }
        let pair = Pair::from_seed(&<Pair as SubPair>::Seed::from(*dk.expose_secret()));
        self.keystore.initialize(&dk, &password)?;
        Ok(pair.public().into())
    }
    /// Returns a signer for alice
    pub fn signer(&self) -> Result<PairSigner> {
        // fetch device key from disk every time to make sure account is unlocked.
        let dk = self.keystore.device_key()?;
        Ok(PairSigner::new(Pair::from_seed(
            &<Pair as SubPair>::Seed::from(*dk.expose_secret()),
        )))
    }
    /// Register flat organization
    pub async fn register_flat_org(
        &self,
        sudo: Option<AccountId32>,
        parent_org: Option<u64>,
        constitution: CidBytes,
        members: &[AccountId32],
    ) -> Result<FlatOrgRegisteredEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .register_flat_org_and_watch(&signer, sudo, parent_org, constitution, members)
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
    ) -> Result<WeightedOrgRegisteredEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .register_weighted_org_and_watch(
                &signer,
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
    ) -> Result<SharesIssuedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .issue_shares_and_watch(&signer, organization, &who, shares)
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
    ) -> Result<SharesBurnedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .burn_shares_and_watch(&signer, organization, &who, shares)
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
    ) -> Result<SharesBatchIssuedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .batch_issue_shares_and_watch(&signer, organization, new_accounts)
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
    ) -> Result<SharesBatchBurnedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .batch_burn_shares_and_watch(&signer, organization, old_accounts)
            .await?
            .shares_batch_burned()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Reserves shares for alice
    pub async fn reserve_shares(
        &self,
        org: u64,
        who: &AccountId32,
    ) -> Result<SharesReservedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .reserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_reserved()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Reserves shares for alice
    pub async fn unreserve_shares(
        &self,
        org: u64,
        who: &AccountId32,
    ) -> Result<SharesUnReservedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .unreserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_un_reserved()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Lock shares for alice
    pub async fn lock_shares(
        &self,
        org: u64,
        who: &AccountId32,
    ) -> Result<SharesLockedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .lock_shares_and_watch(&signer, org, who)
            .await?
            .shares_locked()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
    /// Unlock shares for alice
    pub async fn unlock_shares(
        &self,
        org: u64,
        who: &AccountId32,
    ) -> Result<SharesUnlockedEvent<Runtime>> {
        let signer = self.signer()?;
        self.client
            .clone()
            .unlock_shares_and_watch(&signer, org, who)
            .await?
            .shares_unlocked()
            .map_err(|e| substrate_subxt::Error::Codec(e))?
            .ok_or(Error::EventNotFound)
    }
}
