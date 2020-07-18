use crate::error::{
    Error,
    Result,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    bank::Bank,
    bounty::{
        Bounty,
        BountyClient,
    },
    org::Org,
    vote::Vote,
};
use sunshine_bounty_utils::court::ResolutionMetadata;

#[derive(Clone, Debug, Clap)]
pub struct BountyPostCommand {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
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

impl BountyPostCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as System>::BlockNumber: From<u32> + Display,
        <R as Vote>::Signal: From<u64> + Display,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Bank>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: Display,
    {
        let ac_rejection_threshold: Option<R::Signal> =
            if let Some(ac_r_t) = self.ac_rejection_threshold {
                Some(ac_r_t.into())
            } else {
                None
            };
        let ac_duration: Option<R::BlockNumber> =
            if let Some(ac_d) = self.ac_duration {
                Some(ac_d.into())
            } else {
                None
            };
        let acceptance_committee: ResolutionMetadata<
            <R as Org>::OrgId,
            <R as Vote>::Signal,
            <R as System>::BlockNumber,
        > = ResolutionMetadata::new(
            self.ac_org.into(),
            self.ac_passage_threshold.into(),
            ac_rejection_threshold,
            ac_duration,
        );
        let supervision_committee: Option<
            ResolutionMetadata<
                <R as Org>::OrgId,
                <R as Vote>::Signal,
                <R as System>::BlockNumber,
            >,
        > = if let Some(org) = self.sc_org {
            let passage_threshold = self
                .sc_passage_threshold
                .ok_or(Error::PostBountyInputError)?;
            let sc_rejection_threshold: Option<R::Signal> =
                if let Some(sc_r_t) = self.sc_rejection_threshold {
                    Some(sc_r_t.into())
                } else {
                    None
                };
            let sc_duration: Option<R::BlockNumber> =
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
                (*self.repo_owner).to_string(),
                (*self.repo_name).to_string(),
                self.issue_number,
                self.amount_reserved_for_bounty.into(),
                acceptance_committee,
                supervision_committee,
            )
            .await
            .map_err(Error::Client)?;
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

impl BountyApplicationCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bank>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .account_applies_for_bounty(
                self.bounty_id.into(),
                (*self.description).to_string(),
                self.total_amount.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountyTriggerApplicationReviewCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .account_triggers_application_review(
                self.bounty_id.into(),
                self.app_id.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountySudoApproveApplicationCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Vote>::VoteId: Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .account_sudo_approves_application(
                self.bounty_id.into(),
                self.app_id.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountyPollApplicationCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Vote>::VoteId: Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .poll_application(self.bounty_id.into(), self.app_id.into())
            .await
            .map_err(Error::Client)?;
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
    // submission reference
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub amount_requested: u128,
}

impl BountySubmitMilestoneCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bank>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .submit_milestone(
                self.bounty_id.into(),
                self.application_id.into(),
                (*self.repo_owner).to_string(),
                (*self.repo_name).to_string(),
                self.issue_number,
                self.amount_requested.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountyTriggerMilestoneReviewCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .trigger_milestone_review(
                self.bounty_id.into(),
                self.milestone_id.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountySudoApproveMilestoneCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .sudo_approves_milestone(
                self.bounty_id.into(),
                self.milestone_id.into(),
            )
            .await
            .map_err(Error::Client)?;
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

impl BountyPollMilestoneCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .poll_milestone(self.bounty_id.into(), self.milestone_id.into())
            .await
            .map_err(Error::Client)?;
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
