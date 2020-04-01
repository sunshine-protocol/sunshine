use crate::traits::{Approved, GetCurrentVoteIdentifiers};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
#[non_exhaustive]
pub enum VoterYesNoView {
    /// Voted in favor
    InFavor,
    /// Voted against
    Against,
    /// Acknowledged but abstained
    Abstained,
}

impl VoterYesNoView {
    /// Helper method to tell us if a vote is in favor
    pub fn is_in_favor(self) -> bool {
        match self {
            VoterYesNoView::InFavor => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// Binary vote to express for/against with magnitude
pub struct YesNoVote<Signal> {
    pub direction: VoterYesNoView,
    pub magnitude: Signal,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the threshold configuration
/// - weights votes when they are applied to vote state
/// - evaluates passage of vote state
pub struct ThresholdConfig<FineArithmetic> {
    /// Support threshold
    pub passage_threshold_pct: FineArithmetic,
    /// Required turnout
    pub turnout_threshold_pct: FineArithmetic,
}

//the trait bound should be std::ops::Mul<N: From<u32>> or something like this
impl<FineArithmetic: PerThing> ThresholdConfig<FineArithmetic> {
    pub fn new(
        passage_threshold_pct: FineArithmetic,
        turnout_threshold_pct: FineArithmetic,
    ) -> Self {
        ThresholdConfig {
            passage_threshold_pct,
            turnout_threshold_pct,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// More ergonomic to have this as a field inside of VoteState
pub struct VoteThreshold<Signal, BlockNumber> {
    /// The amount of support required to pass the proposal
    passage_threshold: Signal,
    /// The amount of turnout required for any proposal to pass
    turnout_threshold: Signal,
    /// The time at which these values were last updated (due to VoteConfig governance and electorate changes)
    last_updated: BlockNumber,
}

impl<Signal: Parameter, BlockNumber: Parameter> VoteThreshold<Signal, BlockNumber> {
    pub fn new(
        passage_threshold: Signal,
        turnout_threshold: Signal,
        last_updated: BlockNumber,
    ) -> Self {
        VoteThreshold {
            passage_threshold,
            turnout_threshold,
            last_updated,
        }
    }

    pub fn get_last_updated_time(&self) -> BlockNumber {
        self.last_updated.clone()
    }

    pub fn get_passage_threshold(&self) -> Signal {
        self.passage_threshold.clone()
    }

    pub fn get_turnout_threshold(&self) -> Signal {
        self.turnout_threshold.clone()
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of each executive membership proposal's ongoing voting
pub struct VoteState<Signal, BlockNumber> {
    /// Signal in favor
    pub in_favor: Signal,
    /// Signal against
    pub against: Signal,
    /// All signal that votes
    pub turnout: Signal,
    /// The threshold for passage
    pub threshold: VoteThreshold<Signal, BlockNumber>,
    /// The time at which this is initialized (4_TTL_C_l8r)
    pub initialized: BlockNumber,
    /// The time at which this vote state expired
    pub expires: BlockNumber,
}

impl<Signal: Parameter + PartialOrd, BlockNumber: Parameter> Approved
    for VoteState<Signal, BlockNumber>
{
    fn approved(&self) -> bool {
        self.in_favor > self.threshold.get_passage_threshold()
            && self.turnout > self.threshold.get_turnout_threshold()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
#[non_exhaustive]
/// The vote's state and outcome
pub enum Outcome {
    /// The VoteId in question has been reserved but is not yet open for voting (context is schedule)
    NotStarted,
    /// The VoteState associated with the VoteId is open to voting by the given `ShareId`
    Voting,
    /// The VoteState is approved, thereby unlocking the next `VoteId` if it wraps Some(VoteId)
    Approved,
    /// The VoteState is rejected and all dependent `VoteId`s are not opened
    Rejected,
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::NotStarted
    }
}

impl Approved for Outcome {
    fn approved(&self) -> bool {
        match self {
            Outcome::Approved => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct ScheduledVote<ShareId, FineArithmetic> {
    /// Defines the order relative to other `DispatchableVote`s (impl Ordering)
    priority: u32,
    /// The share type that will be used for this vote
    share_group: ShareId,
    /// The threshold set for this share type in this schedule (TODO: move threshold config out of vote-yesno into here)
    threshold: ThresholdConfig<FineArithmetic>,
}

impl<ShareId: Parameter + Copy, FineArithmetic: PerThing> ScheduledVote<ShareId, FineArithmetic> {
    pub fn new(
        priority: u32,
        share_group: ShareId,
        threshold: ThresholdConfig<FineArithmetic>,
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

    pub fn get_threshold(&self) -> ThresholdConfig<FineArithmetic> {
        self.threshold
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct VoteSchedule<ShareId, VoteId, FineArithmetic> {
    votes_left_including_current: u32,
    current_share_id: ShareId,
    current_vote_id: VoteId,
    schedule: Vec<ScheduledVote<ShareId, FineArithmetic>>,
}

// TODO: are ShareId and VoteId reliably Copy
impl<ShareId: Parameter + Copy, VoteId: Parameter + Copy, FineArithmetic: PerThing>
    VoteSchedule<ShareId, VoteId, FineArithmetic>
{
    /// Note that this object is designed to only be alive while there is a vote dispatched in the vote module
    /// - for this reason, the caller must dispatch the current vote before using the associated identifiers
    /// to instantiate this object
    pub fn new(
        current_share_id: ShareId,
        current_vote_id: VoteId,
        schedule: Vec<ScheduledVote<ShareId, FineArithmetic>>,
    ) -> Self {
        let votes_left_including_current: u32 = (schedule.len() as u32) + 1u32;
        VoteSchedule {
            votes_left_including_current,
            current_share_id,
            current_vote_id,
            schedule,
        }
    }

    // TODO: instead of getters, prefer understanding why the information is gotten and create a method to make
    // the explicit transformation `=>` these getters are equivalent to just making the parameters public
    pub fn get_schedule(self) -> Vec<ScheduledVote<ShareId, FineArithmetic>> {
        self.schedule
    }
    pub fn get_votes_left_including_current(&self) -> u32 {
        self.votes_left_including_current
    }
}

impl<ShareId: Parameter + Copy, VoteId: Parameter + Copy, FineArithmetic: PerThing>
    GetCurrentVoteIdentifiers<ShareId, VoteId> for VoteSchedule<ShareId, VoteId, FineArithmetic>
{
    fn get_current_share_id(&self) -> ShareId {
        self.current_share_id
    }

    fn get_current_vote_id(&self) -> VoteId {
        self.current_vote_id
    }
}
