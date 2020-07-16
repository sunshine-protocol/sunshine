mod subxt;

pub use substrate_subxt::sp_runtime::Permill;
pub use subxt::*;
pub use sunshine_bounty_utils::vote::VoterView;

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
pub trait VoteClient<T: Runtime + Vote>: ChainClient<T> {
    async fn create_signal_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_percent_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_threshold: Permill,
        turnout_threshold: Option<Permill>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<<T as Org>::IpfsReference>,
    ) -> Result<VotedEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> VoteClient<T> for C
where
    T: Runtime + Vote,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::Error: From<Error>,
{
    async fn create_signal_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .create_signal_threshold_vote_and_watch(
                signer,
                topic,
                organization,
                support_requirement,
                turnout_requirement,
                duration,
            )
            .await?
            .new_vote_started()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn create_percent_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_threshold: Permill,
        turnout_threshold: Option<Permill>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .create_percent_threshold_vote_and_watch(
                signer,
                topic,
                organization,
                support_threshold,
                turnout_threshold,
                duration,
            )
            .await?
            .new_vote_started()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .create_unanimous_consent_vote_and_watch(
                signer,
                topic,
                organization,
                duration,
            )
            .await?
            .new_vote_started()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<<T as Org>::IpfsReference>,
    ) -> Result<VotedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .submit_vote_and_watch(signer, vote_id, direction, justification)
            .await?
            .voted()?
            .ok_or(Error::EventNotFound.into())
    }
}
