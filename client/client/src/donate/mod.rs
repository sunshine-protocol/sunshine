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
    Node,
    Result,
};

#[async_trait]
pub trait DonateClient<N: Node>: Client<N>
where
    N::Runtime: Donate,
{
    async fn make_prop_donation(
        &self,
        org: <N::Runtime as Org>::OrgId,
        rem_recipient: <N::Runtime as System>::AccountId,
        amt: BalanceOf<N::Runtime>,
    ) -> Result<PropDonationExecutedEvent<N::Runtime>>;
    async fn make_equal_donation(
        &self,
        org: <N::Runtime as Org>::OrgId,
        rem_recipient: <N::Runtime as System>::AccountId,
        amt: BalanceOf<N::Runtime>,
    ) -> Result<EqualDonationExecutedEvent<N::Runtime>>;
}

#[async_trait]
impl<N, C> DonateClient<N> for C
where
    N: Node,
    N::Runtime: Donate,
    <<<N::Runtime as Runtime>::Extra as SignedExtra<N::Runtime>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<N>,
{
    async fn make_prop_donation(
        &self,
        org: <N::Runtime as Org>::OrgId,
        rem_recipient: <N::Runtime as System>::AccountId,
        amt: BalanceOf<N::Runtime>,
    ) -> Result<PropDonationExecutedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_prop_donation_and_watch(&signer, org, rem_recipient, amt)
            .await?
            .prop_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn make_equal_donation(
        &self,
        org: <N::Runtime as Org>::OrgId,
        rem_recipient: <N::Runtime as System>::AccountId,
        amt: BalanceOf<N::Runtime>,
    ) -> Result<EqualDonationExecutedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .make_equal_donation_and_watch(&signer, org, rem_recipient, amt)
            .await?
            .equal_donation_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
}
