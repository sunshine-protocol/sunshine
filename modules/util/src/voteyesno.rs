use crate::traits::{
    Apply, Approved, DeriveThresholdRequirement, GetCurrentVoteIdentifiers, Revert, VoteVector,
};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub enum VoterYesNoView {
    /// Voted in favor
    InFavor,
    /// Voted against
    Against,
    /// Acknowledged but abstained
    Abstain,
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
/// ~ vectors have direction and magnitude, not to be confused with `Vec`
pub struct YesNoVote<Signal> {
    magnitude: Signal,
    direction: VoterYesNoView,
}

impl<Signal: Parameter> YesNoVote<Signal> {
    pub fn new(magnitude: Signal, direction: VoterYesNoView) -> Self {
        YesNoVote {
            magnitude,
            direction,
        }
    }
}

impl<Signal: Parameter + Copy> VoteVector<Signal, VoterYesNoView> for YesNoVote<Signal> {
    fn magnitude(&self) -> Signal {
        self.magnitude
    }

    fn direction(&self) -> VoterYesNoView {
        self.direction
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the threshold configuration
/// - weights votes when they are applied to vote state
/// - evaluates passage of vote state
pub struct ThresholdConfig<FineArithmetic> {
    /// Support threshold
    passage_threshold_pct: FineArithmetic,
    /// Required turnout
    turnout_threshold_pct: FineArithmetic,
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

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of each executive membership proposal's ongoing voting
pub struct VoteState<Signal, Permill, BlockNumber> {
    /// Signal in favor
    in_favor: Signal,
    /// Signal against
    against: Signal,
    /// All signal that votes
    turnout: Signal,
    /// All signal that could vote
    all_possible_turnout: Signal,
    /// The threshold configuration for passage
    threshold: ThresholdConfig<Permill>,
    /// The time at which this is initialized (4_TTL_C_l8r)
    initialized: BlockNumber,
    /// The time at which this vote state expired
    expires: BlockNumber,
}

impl<
        Signal: Parameter
            + Default
            + Copy
            + sp_std::ops::Add<Signal, Output = Signal>
            + sp_std::ops::Sub<Signal, Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > VoteState<Signal, FineArithmetic, BlockNumber>
{
    pub fn new(
        all_possible_turnout: Signal,
        threshold: ThresholdConfig<FineArithmetic>,
        initialized: BlockNumber,
        expires: BlockNumber,
    ) -> VoteState<Signal, FineArithmetic, BlockNumber> {
        VoteState {
            all_possible_turnout,
            threshold,
            initialized,
            expires,
            ..Default::default()
        }
    }
    pub fn in_favor(&self) -> Signal {
        self.in_favor
    }
    pub fn against(&self) -> Signal {
        self.against
    }
    pub fn turnout(&self) -> Signal {
        self.turnout
    }
    pub fn all_possible_turnout(&self) -> Signal {
        self.all_possible_turnout
    }
    pub fn expires(&self) -> BlockNumber {
        self.expires
    }

    // and turnout
    pub fn add_in_favor_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        let new_in_favor = self.in_favor() + magnitude;
        VoteState {
            in_favor: new_in_favor,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    // and turnout
    pub fn add_against_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        let new_against = self.against() + magnitude;
        VoteState {
            against: new_against,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    // add abstained
    pub fn add_abstention(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        VoteState {
            turnout: new_turnout,
            ..self.clone()
        }
    }
    // remove turnout
    pub fn remove_in_favor_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        let new_in_favor = self.in_favor() - magnitude;
        VoteState {
            in_favor: new_in_favor,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    // remove turnout
    pub fn remove_against_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        let new_against = self.against() - magnitude;
        VoteState {
            against: new_against,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    // remove abstained
    pub fn remove_abstention(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        VoteState {
            turnout: new_turnout,
            ..self.clone()
        }
    }
}

impl<
        Signal: Parameter + sp_std::ops::Mul<Signal, Output = Signal>,
        FineArithmetic: PerThing + sp_std::ops::Mul<Signal, Output = Signal>,
    > DeriveThresholdRequirement<Signal> for ThresholdConfig<FineArithmetic>
{
    fn derive_support_requirement(&self, turnout: Signal) -> Signal {
        self.passage_threshold_pct * turnout
    }
    fn derive_turnout_requirement(&self, turnout: Signal) -> Signal {
        self.turnout_threshold_pct * turnout
    }
}

impl<
        Signal: Parameter
            + PartialOrd
            + Default
            + Copy
            + sp_std::ops::Add<Signal, Output = Signal>
            + sp_std::ops::Sub<Signal, Output = Signal>
            + sp_std::ops::Mul<Signal, Output = Signal>,
        FineArithmetic: PerThing + Default + sp_std::ops::Mul<Signal, Output = Signal>,
        BlockNumber: Parameter + Copy + Default,
    > Approved for VoteState<Signal, FineArithmetic, BlockNumber>
{
    fn approved(&self) -> bool {
        self.in_favor()
            > self
                .threshold
                .derive_support_requirement(self.all_possible_turnout())
            && self.turnout()
                > self
                    .threshold
                    .derive_turnout_requirement(self.all_possible_turnout())
    }
}

impl<
        Signal: Parameter
            + Copy
            + Default
            + sp_std::ops::Sub<Signal, Output = Signal>
            + sp_std::ops::Add<Signal, Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > Apply<YesNoVote<Signal>> for VoteState<Signal, FineArithmetic, BlockNumber>
{
    fn apply(&self, vote: YesNoVote<Signal>) -> VoteState<Signal, FineArithmetic, BlockNumber> {
        match vote.direction() {
            VoterYesNoView::InFavor => self.add_in_favor_vote(vote.magnitude()),
            VoterYesNoView::Against => self.add_against_vote(vote.magnitude()),
            VoterYesNoView::Abstain => self.add_abstention(vote.magnitude()),
        }
    }
}

impl<
        Signal: Parameter
            + Copy
            + Default
            + sp_std::ops::Sub<Signal, Output = Signal>
            + sp_std::ops::Add<Signal, Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > Revert<YesNoVote<Signal>> for VoteState<Signal, FineArithmetic, BlockNumber>
{
    fn revert(&self, vote: YesNoVote<Signal>) -> VoteState<Signal, FineArithmetic, BlockNumber> {
        match vote.direction() {
            VoterYesNoView::InFavor => self.remove_in_favor_vote(vote.magnitude()),
            VoterYesNoView::Against => self.remove_against_vote(vote.magnitude()),
            VoterYesNoView::Abstain => self.remove_abstention(vote.magnitude()),
        }
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
