mod subxt;

pub use subxt::*;

use crate::{
    error::Error,
    org::Org,
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
pub trait DonateClient<T: Runtime + Donate>: ChainClient<T> {
    async fn make_prop_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<PropDonationExecutedEvent<T>, Self::Error>;
    async fn make_equal_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<EqualDonationExecutedEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> DonateClient<T> for C
where
    T: Runtime + Donate,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::Error: From<Error>,
{
    async fn make_prop_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<PropDonationExecutedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_prop_donation_and_watch(signer, org, rem_recipient, amt)
            .await?
            .prop_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn make_equal_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<EqualDonationExecutedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_equal_donation_and_watch(signer, org, rem_recipient, amt)
            .await?
            .equal_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
}
