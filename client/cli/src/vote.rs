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
    sp_runtime::Permill,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    org::Org,
    vote::{
        Vote,
        VoteClient,
    },
    Error as E,
    TextBlock,
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
        <R as Vote>::VoteTopic: From<TextBlock>,
    {
        let topic: Option<<R as Vote>::VoteTopic> = if let Some(t) = &self.topic
        {
            Some(
                TextBlock {
                    text: (*t).to_string(),
                }
                .into(),
            )
        } else {
            None
        };
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
                topic,
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

fn u8_to_permill(u: u8) -> Result<Permill, Error<E>> {
    if u > 0u8 && u < 100u8 {
        Ok(Permill::from_percent(u.into()))
    } else {
        Err(Error::VotePercentThresholdInputBoundError)
    }
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
        <R as Vote>::VoteTopic: From<TextBlock>,
        <R as Vote>::Percent: From<Permill>,
    {
        let topic: Option<<R as Vote>::VoteTopic> = if let Some(t) = &self.topic
        {
            Some(
                TextBlock {
                    text: (*t).to_string(),
                }
                .into(),
            )
        } else {
            None
        };
        let duration: Option<<R as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        let support_threshold: <R as Vote>::Percent =
            u8_to_permill(self.support_threshold)
                .map_err(|_| Error::VotePercentThresholdInputBoundError)?
                .into();
        let turnout_threshold: Option<<R as Vote>::Percent> =
            if let Some(req) = self.turnout_threshold {
                let ret = u8_to_permill(req)
                    .map_err(|_| Error::VotePercentThresholdInputBoundError)?;
                Some(ret.into())
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
        <R as Vote>::VoteTopic: From<TextBlock>,
    {
        let topic: Option<<R as Vote>::VoteTopic> = if let Some(t) = &self.topic
        {
            Some(
                TextBlock {
                    text: (*t).to_string(),
                }
                .into(),
            )
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
            .create_unanimous_consent_vote(
                topic,
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
        <R as Vote>::VoterView: From<VoterView>,
        <R as Vote>::VoteJustification: From<TextBlock>,
    {
        let voter_view: <R as Vote>::VoterView = match self.direction {
            0u8 => VoterView::Against, // 0 == false
            1u8 => VoterView::InFavor, // 1 == true
            _ => VoterView::Abstain,
        }
        .into();
        let justification: Option<<R as Vote>::VoteJustification> =
            if let Some(j) = &self.justification {
                Some(
                    TextBlock {
                        text: (*j).to_string(),
                    }
                    .into(),
                )
            } else {
                None
            };
        let event = client
            .submit_vote(self.vote_id.into(), voter_view, justification)
            .await
            .map_err(Error::Client)?;
        println!(
            "Account {} voted with view {:?} in VoteId {}",
            event.voter, event.view, event.vote_id
        );
        Ok(())
    }
}
