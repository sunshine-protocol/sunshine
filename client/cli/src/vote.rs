use crate::{
    async_trait,
    error::Error,
    AbstractClient,
    Bank,
    Bounty,
    Command,
    Donate,
    Org,
    Pair,
    Permill,
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
pub struct VoteCreateSignalThresholdCommand {
    pub topic: Option<String>,
    pub organization: u64,
    pub support_requirement: u64,
    pub turnout_requirement: Option<u64>,
    pub duration: Option<u32>,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for VoteCreateSignalThresholdCommand
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
            .create_signal_threshold_vote(
                topic,
                self.organization.into(),
                self.support_requirement.into(),
                turnout_requirement,
                duration,
            )
            .await?;
        println!(
            "Account {} created a signal threshold vote for OrgId {} with VoteId {}",
            event.caller, event.org, event.new_vote_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct VoteCreatePercentThresholdCommand {
    pub topic: Option<String>,
    pub organization: u64,
    pub support_threshold: u8,
    pub turnout_threshold: Option<u8>,
    pub duration: Option<u32>,
}

fn u8_to_permill(u: u8) -> Result<Permill> {
    if u > 0u8 && u < 100u8 {
        Ok(Permill::from_percent(u.into()))
    } else {
        Err(Error::VotePercentThresholdInputBoundError)
    }
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for VoteCreatePercentThresholdCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as System>::BlockNumber: From<u32> + Display,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
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
        let support_threshold = u8_to_permill(self.support_threshold)?;
        let turnout_threshold: Option<Permill> =
            if let Some(req) = self.turnout_threshold {
                let ret = u8_to_permill(req)?;
                Some(ret)
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
            .create_percent_threshold_vote(
                topic,
                self.organization.into(),
                support_threshold,
                turnout_threshold,
                duration,
            )
            .await?;
        println!(
            "Account {} created a percent threshold vote for OrgId {} with VoteId {}",
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
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
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
            .create_unanimous_consent_vote(
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
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for VoteSubmitCommand
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
