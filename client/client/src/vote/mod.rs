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
pub trait VoteClient<T: Runtime + Vote>: ChainClient<T> {
    async fn create_signal_threshold_vote_weighted(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_signal_threshold_vote_flat(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_percent_threshold_vote_weighted(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_threshold: <T as Vote>::Percent,
        turnout_threshold: Option<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_percent_threshold_vote_flat(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_threshold: <T as Vote>::Percent,
        turnout_threshold: Option<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: <T as Vote>::VoterView,
        justification: Option<<T as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> VoteClient<T> for C
where
    T: Runtime + Vote,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::IpfsReference: From<libipld::cid::Cid>,
    C: ChainClient<T>,
    C::Error: From<Error>,
    C::OffchainClient: ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Vote>::VoteTopic,
        > + ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Vote>::VoteJustification,
        >,
{
    async fn create_signal_threshold_vote_weighted(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, t).await?)
        } else {
            None
        };
        self.chain_client()
            .create_signal_threshold_vote_weighted_and_watch(
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
    async fn create_signal_threshold_vote_flat(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, t).await?)
        } else {
            None
        };
        self.chain_client()
            .create_signal_threshold_vote_flat_and_watch(
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
    async fn create_percent_threshold_vote_weighted(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_threshold: <T as Vote>::Percent,
        turnout_threshold: Option<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, t).await?)
        } else {
            None
        };
        self.chain_client()
            .create_percent_threshold_vote_weighted_and_watch(
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
    async fn create_percent_threshold_vote_flat(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        support_threshold: <T as Vote>::Percent,
        turnout_threshold: Option<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, t).await?)
        } else {
            None
        };
        self.chain_client()
            .create_percent_threshold_vote_flat_and_watch(
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
        topic: Option<<T as Vote>::VoteTopic>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, t).await?)
        } else {
            None
        };
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
        direction: <T as Vote>::VoterView,
        justification: Option<<T as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let justification = if let Some(j) = justification {
            Some(crate::post(self, j).await?)
        } else {
            None
        };
        self.chain_client()
            .submit_vote_and_watch(signer, vote_id, direction, justification)
            .await?
            .voted()?
            .ok_or(Error::EventNotFound.into())
    }
}
