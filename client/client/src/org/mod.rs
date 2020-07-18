mod subxt;
mod utils;

pub use subxt::*;
pub use utils::AccountShare;

use crate::{
    error::Error,
    TextBlock,
};
use async_trait::async_trait;
use substrate_subxt::{
    system::System,
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[async_trait]
pub trait OrgClient<T: Runtime + Org>: ChainClient<T> {
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: String,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>, Self::Error>;
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: String,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>, Self::Error>;
    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>, Self::Error>;
    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>, Self::Error>;
    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>, Self::Error>;
    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>, Self::Error>;
    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>, Self::Error>;
    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>, Self::Error>;
    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>, Self::Error>;
    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> OrgClient<T> for C
where
    T: Runtime + Org,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::IpfsReference: From<libipld::cid::Cid>,
    C: ChainClient<T>,
    C::Error: From<Error>,
    C::OffchainClient:
        ipld_block_builder::Cache<ipld_block_builder::Codec, TextBlock>,
{
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: String,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let constitution =
            crate::post_text(self, TextBlock { text: constitution }).await?;
        self.chain_client()
            .register_flat_org_and_watch(
                signer,
                sudo,
                parent_org,
                constitution,
                members,
            )
            .await?
            .new_flat_organization_registered()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: String,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let constitution =
            crate::post_text(self, TextBlock { text: constitution }).await?;
        self.chain_client()
            .register_weighted_org_and_watch(
                signer,
                sudo,
                parent_org,
                constitution,
                weighted_members,
            )
            .await?
            .new_weighted_organization_registered()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .issue_shares_and_watch(signer, organization, &who, shares)
            .await?
            .shares_issued()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .burn_shares_and_watch(signer, organization, &who, shares)
            .await?
            .shares_burned()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_issue_shares_and_watch(signer, organization, new_accounts)
            .await?
            .shares_batch_issued()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_burn_shares_and_watch(signer, organization, old_accounts)
            .await?
            .shares_batch_burned()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .reserve_shares_and_watch(signer, org, who)
            .await?
            .shares_reserved()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .unreserve_shares_and_watch(signer, org, who)
            .await?
            .shares_un_reserved()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .lock_shares_and_watch(signer, org, who)
            .await?
            .shares_locked()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .unlock_shares_and_watch(signer, org, who)
            .await?
            .shares_unlocked()?
            .ok_or(Error::EventNotFound.into())
    }
}
