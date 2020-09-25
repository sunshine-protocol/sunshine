mod subxt;

pub use subxt::*;

use crate::{
    error::Error,
    org::Org,
};
use libipld::{
    cache::Cache,
    cbor::DagCborCodec,
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
    Node,
    OffchainConfig,
    Result,
};

#[async_trait]
pub trait VoteClient<N: Node>: Client<N>
where
    N::Runtime: Vote,
{
    async fn create_signal_vote(
        &self,
        topic: Option<<N::Runtime as Vote>::VoteTopic>,
        organization: OrgRep<<N::Runtime as Org>::OrgId>,
        threshold: Threshold<<N::Runtime as Vote>::Signal>,
        duration: Option<<N::Runtime as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<N::Runtime>>;
    async fn create_percent_vote(
        &self,
        topic: Option<<N::Runtime as Vote>::VoteTopic>,
        organization: OrgRep<<N::Runtime as Org>::OrgId>,
        threshold: Threshold<<N::Runtime as Vote>::Percent>,
        duration: Option<<N::Runtime as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<N::Runtime>>;
    async fn submit_vote(
        &self,
        vote_id: <N::Runtime as Vote>::VoteId,
        direction: <N::Runtime as Vote>::VoterView,
        justification: Option<<N::Runtime as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<N::Runtime>>;
    async fn vote_threshold(
        &self,
        threshold_id: <N::Runtime as Vote>::ThresholdId,
    ) -> Result<ThreshConfig<N::Runtime>>;
}

#[async_trait]
impl<N, C> VoteClient<N> for C
where
    N: Node,
    N::Runtime: Vote,
    <<<N::Runtime as Runtime>::Extra as SignedExtra<N::Runtime>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <N::Runtime as Org>::Cid: From<libipld::cid::Cid>,
    C: Client<N>,
    C::OffchainClient: libipld::cache::Cache<
            OffchainConfig<N>,
            DagCborCodec,
            <N::Runtime as Vote>::VoteTopic,
        > + libipld::cache::Cache<
            OffchainConfig<N>,
            DagCborCodec,
            <N::Runtime as Vote>::VoteJustification,
        >,
{
    async fn create_signal_vote(
        &self,
        topic: Option<<N::Runtime as Vote>::VoteTopic>,
        organization: OrgRep<<N::Runtime as Org>::OrgId>,
        threshold: Threshold<<N::Runtime as Vote>::Signal>,
        duration: Option<<N::Runtime as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(self.offchain_client().insert(t).await?.into())
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
        topic: Option<<N::Runtime as Vote>::VoteTopic>,
        organization: OrgRep<<N::Runtime as Org>::OrgId>,
        threshold: Threshold<<N::Runtime as Vote>::Percent>,
        duration: Option<<N::Runtime as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(self.offchain_client().insert(t).await?.into())
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
        vote_id: <N::Runtime as Vote>::VoteId,
        direction: <N::Runtime as Vote>::VoterView,
        justification: Option<<N::Runtime as Vote>::VoteJustification>,
    ) -> Result<VotedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        let justification = if let Some(j) = justification {
            Some(self.offchain_client().insert(j).await?.into())
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
        threshold_id: <N::Runtime as Vote>::ThresholdId,
    ) -> Result<ThreshConfig<N::Runtime>> {
        Ok(self
            .chain_client()
            .vote_thresholds(threshold_id, None)
            .await?)
    }
}
