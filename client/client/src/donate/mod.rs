mod subxt;

pub use subxt::*;

use crate::{
    error::Error,
    org::Org,
};
use substrate_subxt::{
    system::System,
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_client_utils::{
    async_trait,
    Client,
    Result,
};

#[async_trait]
pub trait DonateClient<T: Runtime + Donate>: Client<T> {
    async fn make_prop_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<PropDonationExecutedEvent<T>>;
    async fn make_equal_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<EqualDonationExecutedEvent<T>>;
}

#[async_trait]
impl<T, C> DonateClient<T> for C
where
    T: Runtime + Donate,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<T>,
{
    async fn make_prop_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<PropDonationExecutedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_prop_donation_and_watch(&signer, org, rem_recipient, amt)
            .await?
            .prop_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn make_equal_donation(
        &self,
        org: <T as Org>::OrgId,
        rem_recipient: <T as System>::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<EqualDonationExecutedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_equal_donation_and_watch(&signer, org, rem_recipient, amt)
            .await?
            .equal_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
}
