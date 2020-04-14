use crate::{traits::GetCurrentVoteIdentifiers, voteyesno::ThresholdConfig};
use codec::{Decode, Encode, FullCodec};
use frame_support::Parameter;
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the outcome when polling an `(OrgId<T>, ProposalIndex)` in [`bank`](../../bank/index.html)
pub enum SimplePollingOutcome<VoteId> {
    /// Moved from the current VoteId to a new VoteId
    MovedToNextVote(VoteId, VoteId),
    /// The current `VoteId` stays the same, voting continues on this current vote_id
    StayedOnCurrentVote(VoteId),
    /// The proposal was approved (change ProposalStage)
    Approved,
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// A vote that is awaiting dispatch from within a `VoteSchedule
/// TODO: consider making associated type `Threshold` and define behavior required \forall thresholds
pub struct ScheduledVote<ShareId, Signal, FineArithmetic>
where
    Signal: FullCodec + Parameter + From<u32>,
    FineArithmetic: PerThing,
{
    /// Defines the order relative to other `DispatchableVote`s (impl Ordering)
    priority: u32,
    /// The share type that will be used for this vote
    share_group: ShareId,
    /// The threshold set for this share type in this schedule (TODO: move threshold config out of vote-yesno into here)
    threshold: ThresholdConfig<Signal, FineArithmetic>,
}

impl<
        ShareId: Parameter + Copy,
        Signal: FullCodec + Parameter + From<u32>,
        FineArithmetic: PerThing,
    > ScheduledVote<ShareId, Signal, FineArithmetic>
{
    pub fn new(
        priority: u32,
        share_group: ShareId,
        threshold: ThresholdConfig<Signal, FineArithmetic>,
    ) -> Self {
        Self {
            priority,
            share_group,
            threshold,
        }
    }
    // TODO: instead of getters, prefer understanding why the information is gotten and create a method to make
    // the explicit transformation `=>` these getters are equivalent to just making the parameters public
    pub fn get_share_id(&self) -> ShareId {
        self.share_group
    }

    pub fn get_threshold(&self) -> ThresholdConfig<Signal, FineArithmetic> {
        self.clone().threshold
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The sequence of group approvals with thresholds
pub struct VoteSchedule<ShareId, VoteId, Signal, FineArithmetic>
where
    Signal: FullCodec + Parameter + From<u32>,
    FineArithmetic: PerThing,
{
    /// The number of votes left before the corresponding proposal is approved
    votes_left_including_current: u32,
    /// The share identifier for the live vote on which the VoteSchedule is awaiting passage
    current_share_id: ShareId,
    /// The vote identifier for the live vote on which the VoteSchedule is awaiting passage
    current_vote_id: VoteId,
    /// The sequence of votes awaiting dispatch upon the passage of the current vote
    schedule: Vec<ScheduledVote<ShareId, Signal, FineArithmetic>>,
}

impl<
        ShareId: Parameter + Copy,
        VoteId: Parameter + Copy,
        Signal: FullCodec + Parameter + From<u32>,
        FineArithmetic: PerThing,
    > VoteSchedule<ShareId, VoteId, Signal, FineArithmetic>
{
    /// Note that this object is designed to only be alive while there is a vote dispatched in the vote module
    /// - for this reason, the caller must dispatch the current vote before using the associated identifiers
    /// to instantiate this object
    pub fn new(
        current_share_id: ShareId,
        current_vote_id: VoteId,
        schedule: Vec<ScheduledVote<ShareId, Signal, FineArithmetic>>,
    ) -> Self {
        let votes_left_including_current: u32 = (schedule.len() as u32) + 1u32;
        VoteSchedule {
            votes_left_including_current,
            current_share_id,
            current_vote_id,
            schedule,
        }
    }
    pub fn get_schedule(self) -> Vec<ScheduledVote<ShareId, Signal, FineArithmetic>> {
        self.schedule
    }
    pub fn get_votes_left_including_current(&self) -> u32 {
        self.votes_left_including_current
    }
}

impl<
        ShareId: Parameter + Copy,
        VoteId: Parameter + Copy,
        Signal: FullCodec + Parameter + From<u32>,
        FineArithmetic: PerThing,
    > GetCurrentVoteIdentifiers<ShareId, VoteId>
    for VoteSchedule<ShareId, VoteId, Signal, FineArithmetic>
{
    fn get_current_share_id(&self) -> ShareId {
        self.current_share_id
    }

    fn get_current_vote_id(&self) -> VoteId {
        self.current_vote_id
    }
}
