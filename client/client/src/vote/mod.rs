mod subxt;

pub use subxt::*;
pub use sunshine_bounty_utils::vote::VoterView;

use crate::{
    error::Error,
    org::Org,
    TextBlock,
};
use async_trait::async_trait;
use substrate_subxt::{
    sp_runtime::Permill,
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
        topic: Option<String>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_percent_threshold_vote(
        &self,
        topic: Option<String>,
        organization: T::OrgId,
        support_threshold: u8,
        turnout_threshold: Option<u8>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<String>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, Self::Error>;
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<String>,
    ) -> Result<VotedEvent<T>, Self::Error>;
}

fn u8_to_permill(u: u8) -> Result<Permill, Error> {
    if u > 0u8 && u < 100u8 {
        Ok(Permill::from_percent(u.into()))
    } else {
        Err(Error::VotePercentThresholdInputBoundError)
    }
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
    C::OffchainClient:
        ipld_block_builder::Cache<ipld_block_builder::Codec, TextBlock>,
{
    async fn create_signal_threshold_vote(
        &self,
        topic: Option<String>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, TextBlock { text: t }).await?)
        } else {
            None
        };
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
        topic: Option<String>,
        organization: T::OrgId,
        support_threshold: u8,
        turnout_threshold: Option<u8>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, TextBlock { text: t }).await?)
        } else {
            None
        };
        let support_threshold = u8_to_permill(support_threshold)
            .map_err(|_| Error::VotePercentThresholdInputBoundError)?;
        let turnout_threshold: Option<Permill> =
            if let Some(req) = turnout_threshold {
                let ret = u8_to_permill(req)
                    .map_err(|_| Error::VotePercentThresholdInputBoundError)?;
                Some(ret)
            } else {
                None
            };
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
        topic: Option<String>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let topic = if let Some(t) = topic {
            Some(crate::post(self, TextBlock { text: t }).await?)
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
        direction: VoterView,
        justification: Option<String>,
    ) -> Result<VotedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let justification = if let Some(j) = justification {
            Some(crate::post(self, TextBlock { text: j }).await?)
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
