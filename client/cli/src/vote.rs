use crate::error::VotePercentThresholdInputBoundError;
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
    TextBlock,
};
use sunshine_bounty_utils::{
    organization::OrgRep,
    vote::{
        Threshold,
        VoterView,
    },
};
use sunshine_client_utils::Result;

#[derive(Clone, Debug, Clap)]
pub struct VoteCreateSignalThresholdCommand {
    pub topic: Option<String>,
    pub weighted: u8,
    pub organization: u64,
    pub support_requirement: u64,
    pub rejection_requirement: Option<u64>,
    pub duration: Option<u32>,
}

impl VoteCreateSignalThresholdCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
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
        let rt: Option<R::Signal> = if let Some(r) = self.rejection_requirement
        {
            Some(r.into())
        } else {
            None
        };
        let threshold: Threshold<R::Signal> =
            Threshold::new(self.support_requirement.into(), rt);
        let duration: Option<<R as System>::BlockNumber> =
            if let Some(req) = self.duration {
                Some(req.into())
            } else {
                None
            };
        // 0 is false, every other integer is true
        let event = if self.weighted != 0 {
            client
                .create_signal_vote(
                    topic,
                    OrgRep::Weighted(self.organization.into()),
                    threshold,
                    duration,
                )
                .await?
        } else {
            client
                .create_signal_vote(
                    topic,
                    OrgRep::Equal(self.organization.into()),
                    threshold,
                    duration,
                )
                .await?
        };
        println!(
            "Account {} created a signal threshold vote with VoteId {}",
            event.caller, event.new_vote_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct VoteCreatePercentThresholdCommand {
    pub topic: Option<String>,
    pub weighted: u8,
    pub organization: u64,
    pub support_threshold: u8,
    pub rejection_threshold: Option<u8>,
    pub duration: Option<u32>,
}

pub fn u8_to_permill(u: u8) -> Result<Permill> {
    if u > 0u8 && u < 100u8 {
        Ok(Permill::from_percent(u.into()))
    } else {
        Err(VotePercentThresholdInputBoundError.into())
    }
}

impl VoteCreatePercentThresholdCommand {
    pub async fn exec<R: Runtime + Vote, C: VoteClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
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
        let rt: Option<<R as Vote>::Percent> =
            if let Some(r) = self.rejection_threshold {
                let ret = u8_to_permill(r)
                    .map_err(|_| VotePercentThresholdInputBoundError)?;
                Some(ret.into())
            } else {
                None
            };
        let support_t: <R as Vote>::Percent =
            u8_to_permill(self.support_threshold)
                .map_err(|_| VotePercentThresholdInputBoundError)?
                .into();
        let threshold: Threshold<<R as Vote>::Percent> =
            Threshold::new(support_t, rt);
        // 0 is false and everything else is true
        let event = if self.weighted != 0 {
            client
                .create_percent_vote(
                    topic,
                    OrgRep::Weighted(self.organization.into()),
                    threshold,
                    duration,
                )
                .await?
        } else {
            client
                .create_percent_vote(
                    topic,
                    OrgRep::Equal(self.organization.into()),
                    threshold,
                    duration,
                )
                .await?
        };
        println!(
            "Account {} created a percent threshold vote with VoteId {}",
            event.caller, event.new_vote_id
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
    ) -> Result<()>
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
            .await?;
        println!(
            "Account {} voted with view {:?} in VoteId {}",
            event.voter, event.view, event.vote_id
        );
        Ok(())
    }
}
