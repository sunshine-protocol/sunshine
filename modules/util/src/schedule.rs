use crate::traits::GetCurrentVoteIdentifiers;
use codec::{Decode, Encode};
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the outcome when polling an `(OrgId<T>, ProposalIndex)` in [`bank`](../../bank/index.html)
pub enum SimplePollingOutcome {
    /// Moved from the current VoteId to a new VoteId
    MovedToNextVote(u32, u32),
    /// The current `VoteId` stays the same, voting continues on this current vote_id
    StayedOnCurrentVote(u32),
    /// The proposal was approved (change ProposalStage)
    Approved,
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The type for the threshold
pub enum ThresholdTypeBuilder<FineArithmetic>
where
    FineArithmetic: PerThing,
{
    Count(u32),
    Percentage(FineArithmetic),
}

impl<FineArithmetic: PerThing> Default for ThresholdTypeBuilder<FineArithmetic> {
    fn default() -> ThresholdTypeBuilder<FineArithmetic> {
        ThresholdTypeBuilder::Count(1u32)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the threshold configuration
/// - evaluates passage of vote state
pub struct ThresholdConfigBuilder<FineArithmetic>
where
    FineArithmetic: PerThing,
{
    /// Support threshold
    pub support_required: ThresholdTypeBuilder<FineArithmetic>,
    /// Required turnout
    pub turnout_required: ThresholdTypeBuilder<FineArithmetic>,
}

impl<FineArithmetic: PerThing> ThresholdConfigBuilder<FineArithmetic> {
    pub fn new_signal_count_threshold(support_required: u32, turnout_required: u32) -> Self {
        ThresholdConfigBuilder {
            support_required: ThresholdTypeBuilder::Count(support_required),
            turnout_required: ThresholdTypeBuilder::Count(turnout_required),
        }
    }
    pub fn new_percentage_threshold(
        support_required: FineArithmetic,
        turnout_required: FineArithmetic,
    ) -> Self {
        ThresholdConfigBuilder {
            support_required: ThresholdTypeBuilder::Percentage(support_required),
            turnout_required: ThresholdTypeBuilder::Percentage(turnout_required),
        }
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// A vote that is awaiting dispatch from within a `VoteSchedule
/// TODO: consider making associated type `Threshold` and define behavior required \forall thresholds
pub struct ScheduledVote<FineArithmetic>
where
    FineArithmetic: PerThing,
{
    /// Defines the order relative to other `DispatchableVote`s (impl Ordering)
    priority: u32,
    /// The share type that will be used for this vote
    share_group: u32,
    /// The threshold set for this share type in this schedule (TODO: move threshold config out of vote-yesno into here)
    threshold: ThresholdConfigBuilder<FineArithmetic>,
}

impl<FineArithmetic: PerThing> ScheduledVote<FineArithmetic> {
    pub fn new(
        priority: u32,
        share_group: u32,
        threshold: ThresholdConfigBuilder<FineArithmetic>,
    ) -> Self {
        Self {
            priority,
            share_group,
            threshold,
        }
    }
    // TODO: instead of getters, prefer understanding why the information is gotten and create a method to make
    // the explicit transformation `=>` these getters are equivalent to just making the parameters public
    pub fn get_share_id(&self) -> u32 {
        self.share_group
    }

    pub fn get_threshold(&self) -> ThresholdConfigBuilder<FineArithmetic> {
        self.clone().threshold
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The sequence of group approvals with thresholds
pub struct VoteSchedule<FineArithmetic>
where
    FineArithmetic: PerThing,
{
    /// The number of votes left before the corresponding proposal is approved
    votes_left_including_current: u32,
    /// The share identifier for the live vote on which the VoteSchedule is awaiting passage
    current_share_id: u32,
    /// The vote identifier for the live vote on which the VoteSchedule is awaiting passage
    current_vote_id: u32,
    /// The sequence of votes awaiting dispatch upon the passage of the current vote
    schedule: Vec<ScheduledVote<FineArithmetic>>,
}

impl<FineArithmetic: PerThing> VoteSchedule<FineArithmetic> {
    /// Note that this object is designed to only be alive while there is a vote dispatched in the vote module
    /// - for this reason, the caller must dispatch the current vote before using the associated identifiers
    /// to instantiate this object
    pub fn new(
        current_share_id: u32,
        current_vote_id: u32,
        schedule: Vec<ScheduledVote<FineArithmetic>>,
    ) -> Self {
        let votes_left_including_current: u32 = (schedule.len() as u32) + 1u32;
        VoteSchedule {
            votes_left_including_current,
            current_share_id,
            current_vote_id,
            schedule,
        }
    }
    pub fn get_schedule(self) -> Vec<ScheduledVote<FineArithmetic>> {
        self.schedule
    }
    pub fn get_votes_left_including_current(&self) -> u32 {
        self.votes_left_including_current
    }
}

impl<FineArithmetic: PerThing> GetCurrentVoteIdentifiers for VoteSchedule<FineArithmetic> {
    fn get_current_share_id(&self) -> u32 {
        self.current_share_id
    }

    fn get_current_vote_id(&self) -> u32 {
        self.current_vote_id
    }
}
