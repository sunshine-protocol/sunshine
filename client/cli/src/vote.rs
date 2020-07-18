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
    org::Org,
    vote::{
        Vote,
        VoteClient,
    },
};
use sunshine_bounty_utils::vote::VoterView;

#[derive(Clone, Debug, Clap)]
pub struct VoteCreateSignalThresholdCommand {
    pub topic: Option<String>,
    pub organization: u64,
    pub support_requirement: u64,
    pub turnout_requirement: Option<u64>,
    pub duration: Option<u32>,
}

impl VoteCreateSignalThresholdCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as System>::BlockNumber: From<u32>,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Vote>::Signal: From<u64> + Display,
        <R as Vote>::VoteId: Display,
    {
        let turnout_requirement: Option<R::Signal> =
            if let Some(req) = self.turnout_requirement {
                Some(req.into())
            } else {
                None
            };
        let duration: Option<<R as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let event = client
            .create_signal_threshold_vote(
                self.topic.clone(),
                self.organization.into(),
                self.support_requirement.into(),
                turnout_requirement,
                duration,
            )
            .await
            .map_err(Error::Client)?;
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

impl VoteCreatePercentThresholdCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as System>::BlockNumber: From<u32>,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Vote>::VoteId: Display,
    {
        let duration: Option<<R as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let event = client
            .create_percent_threshold_vote(
                self.topic.clone(),
                self.organization.into(),
                self.support_threshold,
                self.turnout_threshold,
                duration,
            )
            .await
            .map_err(Error::Client)?;
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

impl VoteCreateUnanimousConsentCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Vote>::VoteId: Display,
    {
        let duration: Option<<R as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let event = client
            .create_unanimous_consent_vote(
                self.topic.clone(),
                self.organization.into(),
                duration,
            )
            .await
            .map_err(Error::Client)?;
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

impl VoteSubmitCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Vote>::VoteId: From<u64> + Display,
    {
        let voter_view = match self.direction {
            0u8 => VoterView::Against, // 0 == false
            1u8 => VoterView::InFavor, // 1 == true
            _ => VoterView::Abstain,
        };
        let event = client
            .submit_vote(
                self.vote_id.into(),
                voter_view,
                self.justification.clone(),
            )
            .await
            .map_err(Error::Client)?;
        println!(
            "Account {} voted with view {:?} in VoteId {}",
            event.voter, event.view, event.vote_id
        );
        Ok(())
    }
}
