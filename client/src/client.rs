#[cfg(feature = "light-client")]
use crate::light_client::ChainType;
use crate::{
    error::{
        Error,
        Result,
    },
    srml::org::*,
};
use async_std::sync::{
    Mutex,
    RwLock,
};
use codec::Decode;
use core::marker::PhantomData;
use ipld_block_builder::{
    BlockBuilder,
    Codec,
};
use keystore::{
    DeviceKey,
    KeyStore,
    Password,
};
use libipld::store::Store;
use substrate_subxt::{
    sp_core::crypto::{
        Pair,
        Ss58Codec,
    },
    sp_runtime::traits::{
        IdentifyAccount,
        SignedExtension,
        Verify,
    },
    system::System,
    PairSigner,
    Runtime,
    SignedExtra,
};

pub struct Client<T, P, I>
where
    T: Runtime + Org,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    P: Pair,
    I: Store,
{
    _marker: PhantomData<P>,
    keystore: RwLock<KeyStore>,
    subxt: substrate_subxt::Client<T>,
    pub cache: Mutex<BlockBuilder<I, Codec>>,
}

impl<T, P, I> Client<T, P, I>
where
    T: Runtime + Org,
    <T as System>::AccountId: Into<<T as System>::Address> + Ss58Codec,
    T::Signature: Decode + From<P::Signature>,
    <T::Signature as Verify>::Signer:
        From<P::Public> + IdentifyAccount<AccountId = <T as System>::AccountId>,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    P: Pair,
    <P as Pair>::Public: Into<<T as System>::AccountId>,
    <P as Pair>::Seed: From<[u8; 32]>,
    I: Store,
{
    pub fn new(
        keystore: KeyStore,
        subxt: substrate_subxt::Client<T>,
        store: I,
    ) -> Self {
        Self {
            _marker: PhantomData,
            keystore: RwLock::new(keystore),
            subxt,
            cache: Mutex::new(BlockBuilder::new(store, Codec::new())),
        }
    }
    /// Set device key, directly from substrate-identity to use with keystore
    pub async fn has_device_key(&self) -> bool {
        self.keystore.read().await.is_initialized()
    }
    /// Set device key, directly from substrate-identity to use with keystore
    pub async fn set_device_key(
        &self,
        dk: &DeviceKey,
        password: &Password,
        force: bool,
    ) -> Result<<T as System>::AccountId> {
        if self.keystore.read().await.is_initialized() && !force {
            return Err(Error::KeystoreInitialized)
        }
        let pair = P::from_seed(&P::Seed::from(*dk.expose_secret()));
        self.keystore.write().await.initialize(&dk, &password)?;
        Ok(pair.public().into())
    }
    /// Returns a signer for alice
    pub async fn signer(&self) -> Result<PairSigner<T, P>> {
        // fetch device key from disk every time to make sure account is unlocked.
        let dk = self.keystore.read().await.device_key()?;
        Ok(PairSigner::new(P::from_seed(&P::Seed::from(
            *dk.expose_secret(),
        ))))
    }
    /// Register flat organization
    pub async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .register_flat_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution,
                members,
            )
            .await?
            .new_flat_organization_registered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Register weighted organization
    pub async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .register_weighted_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution,
                weighted_members,
            )
            .await?
            .new_weighted_organization_registered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Issue shares
    pub async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .issue_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_issued()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Burn shares
    pub async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .burn_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_burned()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Batch issue shares
    pub async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .batch_issue_shares_and_watch(&signer, organization, new_accounts)
            .await?
            .shares_batch_issued()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Batch burn shares
    pub async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .batch_burn_shares_and_watch(&signer, organization, old_accounts)
            .await?
            .shares_batch_burned()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Reserves shares for alice
    pub async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .reserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_reserved()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Reserves shares for alice
    pub async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .unreserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_un_reserved()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Lock shares for alice
    pub async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .lock_shares_and_watch(&signer, org, who)
            .await?
            .shares_locked()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    /// Unlock shares for alice
    pub async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .unlock_shares_and_watch(&signer, org, who)
            .await?
            .shares_unlocked()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
}
