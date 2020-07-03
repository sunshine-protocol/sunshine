use crate::{
    error::Result,
    srml::org::*,
    Client,
};
use async_trait::async_trait;
use codec::Decode;
use keystore::{
    DeviceKey,
    Password,
};
use libipld::store::Store;
use sp_core::crypto::{
    Pair,
    Ss58Codec,
};
use sp_runtime::traits::{
    IdentifyAccount,
    SignedExtension,
    Verify,
};
use substrate_subxt::{
    sp_core,
    sp_runtime,
    system::System,
    Runtime,
    SignedExtra,
    Signer,
};

#[async_trait]
pub trait AbstractClient<T: Runtime + Org, P: Pair>: Send + Sync {
    async fn has_device_key(&self) -> bool;
    async fn set_device_key(
        &self,
        dk: &DeviceKey,
        password: &Password,
        force: bool,
    ) -> Result<T::AccountId>;
    async fn signer(&self) -> Result<Box<dyn Signer<T> + Send + Sync>>;
    async fn lock(&self) -> Result<()>;
    async fn unlock(&self, password: &Password) -> Result<()>;
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>>;
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>>;
    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>>;
    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>>;
    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>>;
    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>>;
    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>>;
    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>>;
    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>>;
    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>>;
    fn subxt(&self) -> &substrate_subxt::Client<T>;
}

#[async_trait]
impl<T, P, I> AbstractClient<T, P> for Client<T, P, I>
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
    I: Store + Send + Sync,
{
    async fn has_device_key(&self) -> bool {
        self.has_device_key().await
    }

    async fn set_device_key(
        &self,
        dk: &DeviceKey,
        password: &Password,
        force: bool,
    ) -> Result<T::AccountId> {
        self.set_device_key(dk, password, force).await
    }

    async fn signer(&self) -> Result<Box<dyn Signer<T> + Send + Sync>> {
        Ok(Box::new(self.signer().await?))
    }

    async fn lock(&self) -> Result<()> {
        self.lock().await
    }

    async fn unlock(&self, password: &Password) -> Result<()> {
        self.unlock(password).await
    }

    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>> {
        self.register_flat_org(sudo, parent_org, constitution, members)
            .await
    }

    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>> {
        self.register_weighted_org(
            sudo,
            parent_org,
            constitution,
            weighted_members,
        )
        .await
    }

    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        self.issue_shares(organization, who, shares).await
    }

    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        self.burn_shares(organization, who, shares).await
    }

    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        self.batch_issue_shares(organization, new_accounts).await
    }

    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        self.batch_burn_shares(organization, old_accounts).await
    }

    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>> {
        self.reserve_shares(org, who).await
    }

    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>> {
        self.unreserve_shares(org, who).await
    }

    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>> {
        self.lock_shares(org, who).await
    }

    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>> {
        self.unlock_shares(org, who).await
    }

    fn subxt(&self) -> &substrate_subxt::Client<T> {
        self.subxt()
    }
}
