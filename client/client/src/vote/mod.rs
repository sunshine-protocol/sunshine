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
use sunshine_bounty_utils::{
    organization::OrgRep,
    vote::Threshold,
};
use sunshine_client_utils::{
    async_trait,
    Client,
    Result,
};

#[async_trait]
pub trait VoteClient<T: Runtime + Vote>: Client<T> {
    async fn create_signal_vote(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: OrgRep<T::OrgId>,
        threshold: Threshold<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>>;
    async fn create_percent_vote(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: OrgRep<T::OrgId>,
        threshold: Threshold<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>>;
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: <T as Vote>::VoterView,
        justification: Option<<T as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<T>>;
    async fn vote_threshold(
        &self,
        threshold_id: <T as Vote>::ThresholdId,
    ) -> Result<ThreshConfig<T>>;
}

#[async_trait]
impl<T, C> VoteClient<T> for C
where
    T: Runtime + Vote,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::Cid: From<libipld::cid::Cid>,
    C: Client<T>,
    C::OffchainClient: ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Vote>::VoteTopic,
        > + ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Vote>::VoteJustification,
        >,
{
    async fn create_signal_vote(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: OrgRep<T::OrgId>,
        threshold: Threshold<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            let iref: <T as Org>::Cid = crate::post(self, t).await?.into();
            Some(iref)
        } else {
            None
        };
        self.chain_client()
            .create_signal_vote_and_watch(
                &signer,
                topic,
                organization,
                threshold,
                duration,
            )
            .await?
            .new_vote_started()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn create_percent_vote(
        &self,
        topic: Option<<T as Vote>::VoteTopic>,
        organization: OrgRep<T::OrgId>,
        threshold: Threshold<<T as Vote>::Percent>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            let iref: <T as Org>::Cid = crate::post(self, t).await?.into();
            Some(iref)
        } else {
            None
        };
        self.chain_client()
            .create_percent_vote_and_watch(
                &signer,
                topic,
                organization,
                threshold,
                duration,
            )
            .await?
            .new_vote_started()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: <T as Vote>::VoterView,
        justification: Option<<T as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<T>> {
        let signer = self.chain_signer()?;
        let justification = if let Some(j) = justification {
            let iref: <T as Org>::Cid = crate::post(self, j).await?.into();
            Some(iref)
        } else {
            None
        };
        self.chain_client()
            .submit_vote_and_watch(&signer, vote_id, direction, justification)
            .await?
            .voted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn vote_threshold(
        &self,
        threshold_id: <T as Vote>::ThresholdId,
    ) -> Result<ThreshConfig<T>> {
        Ok(self
            .chain_client()
            .vote_thresholds(threshold_id, None)
            .await?)
    }
}
