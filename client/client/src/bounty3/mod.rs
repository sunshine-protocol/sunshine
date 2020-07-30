mod subxt;

pub use subxt::*;
pub use sunshine_bounty_utils::bounty3::*;

use crate::error::Error;
use async_trait::async_trait;
use substrate_subxt::{
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[async_trait]
pub trait BountyClient<T: Runtime + Bounty3>: ChainClient<T> {
    async fn post_bounty(
        &self,
        info: T::IpfsReference,
        amount: BalanceOf<T>,
    ) -> Result<BountyPostedEvent<T>, Self::Error>;
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>, Self::Error>;
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission_ref: T::IpfsReference,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>, Self::Error>;
    async fn approve_bounty_submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<BountyPaymentExecutedEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> BountyClient<T> for C
where
    T: Runtime + Bounty3,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Bounty3>::IpfsReference: From<libipld::cid::Cid>,
    C: ChainClient<T>,
    C::Error: From<Error>,
    C::OffchainClient: ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Bounty3>::BountyPost,
        > + ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Bounty3>::BountySubmission,
        >,
{
    async fn post_bounty(
        &self,
        info: T::IpfsReference,
        amount: BalanceOf<T>,
    ) -> Result<BountyPostedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .post_bounty_and_watch(signer, info, amount)
            .await?
            .bounty_posted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .contribute_to_bounty_and_watch(signer, bounty_id, amount)
            .await?
            .bounty_raise_contribution()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission_ref: T::IpfsReference,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .submit_for_bounty_and_watch(
                signer,
                bounty_id,
                submission_ref,
                amount,
            )
            .await?
            .bounty_submission_posted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn approve_bounty_submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<BountyPaymentExecutedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .approve_bounty_submission_and_watch(signer, submission_id)
            .await?
            .bounty_payment_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
}
