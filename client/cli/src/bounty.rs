use crate::{
    async_trait,
    AbstractClient,
    Bank,
    Bounty,
    Command,
    Donate,
    Error,
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
use util::court::ResolutionMetadata;
use utils_identity::cid::CidBytes;

#[derive(Clone, Debug, Clap)]
pub struct BountyPostCommand {
    pub description: String,
    pub amount_reserved_for_bounty: u128,
    // ac == acceptance committee
    pub ac_org: u64,
    pub ac_passage_threshold: u64,
    pub ac_rejection_threshold: Option<u64>,
    pub ac_duration: Option<u32>,
    // sc == supervision committee
    pub sc_org: Option<u64>,
    pub sc_passage_threshold: Option<u64>,
    pub sc_rejection_threshold: Option<u64>,
    pub sc_duration: Option<u32>,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyPostCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as System>::BlockNumber: From<u32> + Display,
    <T as Vote>::Signal: From<u64> + Display,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Bank>::Currency: From<u128> + Display,
    <T as Bounty>::BountyId: Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let description: CidBytes = {
            let content = self.description.as_bytes();
            let hash = Blake2b256::digest(&content[..]);
            let cid = Cid::new_v1(Codec::Raw, hash);
            CidBytes::from(&cid)
        };
        let ac_rejection_threshold: Option<T::Signal> =
            if let Some(ac_r_t) = self.ac_rejection_threshold {
                Some(ac_r_t.into())
            } else {
                None
            };
        let ac_duration: Option<T::BlockNumber> =
            if let Some(ac_d) = self.ac_duration {
                Some(ac_d.into())
            } else {
                None
            };
        let acceptance_committee: ResolutionMetadata<
            <T as Org>::OrgId,
            <T as Vote>::Signal,
            <T as System>::BlockNumber,
        > = ResolutionMetadata::new(
            self.ac_org.into(),
            self.ac_passage_threshold.into(),
            ac_rejection_threshold,
            ac_duration,
        );
        let supervision_committee: Option<
            ResolutionMetadata<
                <T as Org>::OrgId,
                <T as Vote>::Signal,
                <T as System>::BlockNumber,
            >,
        > = if let Some(org) = self.sc_org {
            let passage_threshold = self
                .sc_passage_threshold
                .ok_or(Error::PostBountyInputError)?;
            let sc_rejection_threshold: Option<T::Signal> =
                if let Some(sc_r_t) = self.sc_rejection_threshold {
                    Some(sc_r_t.into())
                } else {
                    None
                };
            let sc_duration: Option<T::BlockNumber> =
                if let Some(sc_d) = self.sc_duration {
                    Some(sc_d.into())
                } else {
                    None
                };
            Some(ResolutionMetadata::new(
                org.into(),
                passage_threshold.into(),
                sc_rejection_threshold,
                sc_duration,
            ))
        } else {
            None
        };
        let event = client
            .account_posts_bounty(
                description.into(),
                self.amount_reserved_for_bounty.into(),
                acceptance_committee,
                supervision_committee,
            )
            .await?;
        println!(
            "AccountId {} posted new bounty with identifier {} with amount reserved: {}",
            event.poster, event.new_bounty_id, event.amount_reserved_for_bounty
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyApplicationCommand {
    pub bounty_id: u64,
    pub description: String,
    pub total_amount: u128,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyApplicationCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Bank>::Currency: From<u128> + Display,
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let description: CidBytes = {
            let content = self.description.as_bytes();
            let hash = Blake2b256::digest(&content[..]);
            let cid = Cid::new_v1(Codec::Raw, hash);
            CidBytes::from(&cid)
        };
        let event = client
            .account_applies_for_bounty(
                self.bounty_id.into(),
                description.into(),
                self.total_amount.into(),
            )
            .await?;
        println!(
            "AccountId {:?} applied for bounty with identifier {} with application identifier {} for total amount {}",
            event.submitter, event.bounty_id, event.new_grant_app_id, event.total_amount,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyTriggerApplicationReviewCommand {
    pub bounty_id: u64,
    pub app_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyTriggerApplicationReviewCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Vote>::VoteId: Display,
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .account_triggers_application_review(
                self.bounty_id.into(),
                self.app_id.into(),
            )
            .await?;
        println!(
            "AccountId {:?} triggered review for bounty {} application {} with application state {:?}",
            event.trigger, event.bounty_id, event.application_id, event.application_state
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountySudoApproveApplicationCommand {
    pub bounty_id: u64,
    pub app_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountySudoApproveApplicationCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Vote>::VoteId: Display,
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .account_sudo_approves_application(
                self.bounty_id.into(),
                self.app_id.into(),
            )
            .await?;
        println!(
            "AccountId {:?} sudo approved bounty {} application {} with application state {:?}",
            event.sudo, event.bounty_id, event.application_id, event.application_state
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyPollApplicationCommand {
    pub bounty_id: u64,
    pub app_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyPollApplicationCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Vote>::VoteId: Display,
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .poll_application(self.bounty_id.into(), self.app_id.into())
            .await?;
        println!(
            "AccountId {:?} polled bounty {} application {} with application state {:?}",
            event.poller, event.bounty_id, event.application_id, event.application_state
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountySubmitMilestoneCommand {
    pub bounty_id: u64,
    pub application_id: u64,
    pub submission_reference: String,
    pub amount_requested: u128,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountySubmitMilestoneCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
    <T as Bank>::Currency: From<u128> + Display,
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let submission_reference: CidBytes = {
            let content = self.submission_reference.as_bytes();
            let hash = Blake2b256::digest(&content[..]);
            let cid = Cid::new_v1(Codec::Raw, hash);
            CidBytes::from(&cid)
        };
        let event = client
            .submit_milestone(
                self.bounty_id.into(),
                self.application_id.into(),
                submission_reference.into(),
                self.amount_requested.into(),
            )
            .await?;
        println!(
            "AccountId {:?} submitted a milestone for bounty {} application {} milestone {} for amount {}",
            event.submitter, event.bounty_id, event.application_id, event.new_milestone_id, event.amount_requested,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyTriggerMilestoneReviewCommand {
    pub bounty_id: u64,
    pub milestone_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyTriggerMilestoneReviewCommand
where
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .trigger_milestone_review(
                self.bounty_id.into(),
                self.milestone_id.into(),
            )
            .await?;
        println!(
            "AccountId {:?} triggered a milestone review for bounty {} milestone {} with state {:?}",
            event.trigger, event.bounty_id, event.milestone_id, event.milestone_state,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountySudoApproveMilestoneCommand {
    pub bounty_id: u64,
    pub milestone_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountySudoApproveMilestoneCommand
where
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .sudo_approves_milestone(
                self.bounty_id.into(),
                self.milestone_id.into(),
            )
            .await?;
        println!(
            "AccountId {:?} sudo approved bounty {} milestone {} with state {:?}",
            event.sudo, event.bounty_id, event.milestone_id, event.milestone_state,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyPollMilestoneCommand {
    pub bounty_id: u64,
    pub milestone_id: u64,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BountyPollMilestoneCommand
where
    <T as Bounty>::BountyId: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .poll_milestone(self.bounty_id.into(), self.milestone_id.into())
            .await?;
        println!(
            "AccountId {:?} polled bounty {} milestone {} with state {:?}",
            event.poller,
            event.bounty_id,
            event.milestone_id,
            event.milestone_state,
        );
        Ok(())
    }
}
