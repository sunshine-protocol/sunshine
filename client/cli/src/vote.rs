use crate::{
    async_trait,
    AbstractClient,
    Command,
    Org,
    Pair,
    Result,
    Runtime,
    Vote,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use libipld::{
    cid::{
        Cid,
        Codec,
    },
    multihash::Blake2b256,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
};
use util::vote::VoterView;
use utils_identity::cid::CidBytes;

#[derive(Clone, Debug, Clap)]
pub struct VoteCreateThresholdApprovalCommand {
    pub topic: Option<String>,
    pub organization: u64,
    pub support_requirement: u64,
    pub turnout_requirement: Option<u64>,
    pub duration: Option<u32>,
}

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P>
    for VoteCreateThresholdApprovalCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as System>::BlockNumber: From<u32> + Display,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Vote>::Signal: From<u64> + Display,
    <T as Vote>::VoteId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let topic: Option<T::IpfsReference> =
            if let Some(topic_ref) = &self.topic {
                let content = topic_ref.as_bytes();
                let hash = Blake2b256::digest(&content[..]);
                let cid = Cid::new_v1(Codec::Raw, hash);
                Some(CidBytes::from(&cid).into())
            } else {
                None
            };
        let turnout_requirement: Option<T::Signal> =
            if let Some(req) = self.turnout_requirement {
                Some(req.into())
            } else {
                None
            };
        let duration: Option<<T as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let event = client
            .create_threshold_approval_vote(
                topic,
                self.organization.into(),
                self.support_requirement.into(),
                turnout_requirement,
                duration,
            )
            .await?;
        println!(
            "Account {} created a threshold approval vote for OrgId {} with VoteId {}",
            event.caller, event.org, event.new_vote_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct VoteCreateUnanimousConsentCommand {
    pub topic: Option<String>,
    pub organization: u64,
    pub duration: Option<u32>,
}

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P>
    for VoteCreateUnanimousConsentCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as System>::BlockNumber: From<u32> + Display,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Vote>::Signal: From<u64> + Display,
    <T as Vote>::VoteId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let topic: Option<T::IpfsReference> =
            if let Some(topic_ref) = &self.topic {
                let content = topic_ref.as_bytes();
                let hash = Blake2b256::digest(&content[..]);
                let cid = Cid::new_v1(Codec::Raw, hash);
                Some(CidBytes::from(&cid).into())
            } else {
                None
            };
        let duration: Option<<T as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let event = client
            .create_unanimous_consent_approval_vote(
                topic,
                self.organization.into(),
                duration,
            )
            .await?;
        println!(
            "Account {} created a unanimous consent vote for OrgId {} with VoteId {}",
            event.caller, event.org, event.new_vote_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct VoteSubmitCommand {
    pub vote_id: u64,
    pub direction: u8,
    pub justification: Option<String>,
}

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P> for VoteSubmitCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Vote>::VoteId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let justification: Option<T::IpfsReference> =
            if let Some(topic_ref) = &self.justification {
                let content = topic_ref.as_bytes();
                let hash = Blake2b256::digest(&content[..]);
                let cid = Cid::new_v1(Codec::Raw, hash);
                Some(CidBytes::from(&cid).into())
            } else {
                None
            };
        let voter_view = match self.direction {
            0u8 => VoterView::Against, // 0 == false
            1u8 => VoterView::InFavor, // 1 == true
            _ => VoterView::Abstain,
        };
        let event = client
            .submit_vote(self.vote_id.into(), voter_view, justification.into())
            .await?;
        println!(
            "Account {} voted with view {:?} in VoteId {}",
            event.voter, event.view, event.vote_id
        );
        Ok(())
    }
}
