use crate::traits::{
    Apply, Approved, ConsistentThresholdStructure, DeriveThresholdRequirement, Revert, VoteVector,
};
use codec::{Decode, Encode, FullCodec};
use frame_support::Parameter;
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The types of vote weightings supported by default in `vote-yesno`
pub enum SupportedVoteTypes<Signal> {
    /// 1 account 1 vote
    OneAccountOneVote,
    /// Defaults to share weights
    ShareWeighted,
    /// WARNING: this has no restrictions and shouldn't be exposed in any public API
    Custom(Signal),
}

impl<Signal: Parameter> Default for SupportedVoteTypes<Signal> {
    fn default() -> SupportedVoteTypes<Signal> {
        SupportedVoteTypes::ShareWeighted
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The vote-yesno voter options (direction)
pub enum VoterYesNoView {
    /// Voted in favor
    InFavor,
    /// Voted against
    Against,
    /// Acknowledged but abstained
    Abstain,
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

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The type for the threshold
pub enum ThresholdType<Signal, FineArithmetic>
where
    Signal: FullCodec + Parameter,
    FineArithmetic: PerThing,
{
    Count(Signal),
    Percentage(FineArithmetic),
}

impl<Signal: Parameter + From<u32>, FineArithmetic: PerThing> Default
    for ThresholdType<Signal, FineArithmetic>
{
    fn default() -> ThresholdType<Signal, FineArithmetic> {
        ThresholdType::Count(0u32.into())
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the threshold configuration
/// - evaluates passage of vote state
pub struct ThresholdConfig<Signal, FineArithmetic>
where
    Signal: FullCodec + Parameter + From<u32>,
    FineArithmetic: PerThing,
{
    /// Support threshold
    support_required: ThresholdType<Signal, FineArithmetic>,
    /// Required turnout
    turnout_required: ThresholdType<Signal, FineArithmetic>,
}

impl<Signal: FullCodec + Parameter + From<u32>, FineArithmetic: PerThing>
    ThresholdConfig<Signal, FineArithmetic>
{
    pub fn new_signal_count_threshold(support_required: Signal, turnout_required: Signal) -> Self {
        ThresholdConfig {
            support_required: ThresholdType::Count(support_required),
            turnout_required: ThresholdType::Count(turnout_required),
        }
    }
    pub fn new_percentage_threshold(
        support_required: FineArithmetic,
        turnout_required: FineArithmetic,
    ) -> Self {
        ThresholdConfig {
            support_required: ThresholdType::Percentage(support_required),
            turnout_required: ThresholdType::Percentage(turnout_required),
        }
    }
}

impl<Signal: FullCodec + Parameter + From<u32> + Copy, FineArithmetic: PerThing + Copy>
    ConsistentThresholdStructure for ThresholdConfig<Signal, FineArithmetic>
{
    fn is_percentage_threshold(&self) -> bool {
        match (self.support_required, self.turnout_required) {
            (ThresholdType::Percentage(_), ThresholdType::Percentage(_)) => true,
            _ => false,
        }
    }
    fn is_count_threshold(&self) -> bool {
        match (self.support_required, self.turnout_required) {
            (ThresholdType::Count(_), ThresholdType::Count(_)) => true,
            _ => false,
        }
    }
    fn has_consistent_structure(&self) -> bool {
        Self::is_percentage_threshold(self) || Self::is_count_threshold(self)
    }
}

impl<
        Signal: Parameter + sp_std::ops::Mul<Signal, Output = Signal> + From<u32>,
        FineArithmetic: PerThing + sp_std::ops::Mul<Signal, Output = Signal>,
    > DeriveThresholdRequirement<Signal> for ThresholdConfig<Signal, FineArithmetic>
{
    fn derive_support_requirement(&self, turnout: Signal) -> Signal {
        match self.clone().support_required {
            ThresholdType::Count(signal) => signal,
            ThresholdType::Percentage(signal) => signal * turnout,
        }
    }
    fn derive_turnout_requirement(&self, turnout: Signal) -> Signal {
        match self.clone().turnout_required {
            ThresholdType::Count(signal) => signal,
            ThresholdType::Percentage(signal) => signal * turnout,
        }
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of an ongoing vote
pub struct VoteState<Signal, FineArithmetic, BlockNumber>
where
    Signal: FullCodec + Parameter + From<u32>,
    FineArithmetic: PerThing,
{
    /// Signal in favor
    in_favor: Signal,
    /// Signal against
    against: Signal,
    /// All signal that votes
    turnout: Signal,
    /// All signal that could vote
    all_possible_turnout: Signal,
    /// The threshold configuration for passage
    threshold: ThresholdConfig<Signal, FineArithmetic>,
    /// The time at which this is initialized (4_TTL_C_l8r)
    initialized: BlockNumber,
    /// The time at which this vote state expired, now an Option
    expires: Option<BlockNumber>,
}

impl<
        Signal: Parameter
            + Default
            + FullCodec
            + Copy
            + From<u32>
            + sp_std::ops::Add<Signal, Output = Signal>
            + sp_std::ops::Sub<Signal, Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > VoteState<Signal, FineArithmetic, BlockNumber>
{
    pub fn new(
        all_possible_turnout: Signal,
        threshold: ThresholdConfig<Signal, FineArithmetic>,
        initialized: BlockNumber,
        expires: Option<BlockNumber>,
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
    pub fn expires(&self) -> Option<BlockNumber> {
        self.expires
    }
    pub fn threshold(&self) -> ThresholdConfig<Signal, FineArithmetic> {
        self.threshold
    }

    pub fn add_in_favor_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        let new_in_favor = self.in_favor() + magnitude;
        VoteState {
            in_favor: new_in_favor,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    pub fn add_against_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        let new_against = self.against() + magnitude;
        VoteState {
            against: new_against,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    pub fn add_abstention(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() + magnitude;
        VoteState {
            turnout: new_turnout,
            ..self.clone()
        }
    }
    pub fn remove_in_favor_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        let new_in_favor = self.in_favor() - magnitude;
        VoteState {
            in_favor: new_in_favor,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    pub fn remove_against_vote(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        let new_against = self.against() - magnitude;
        VoteState {
            against: new_against,
            turnout: new_turnout,
            ..self.clone()
        }
    }
    pub fn remove_abstention(&self, magnitude: Signal) -> Self {
        let new_turnout = self.turnout() - magnitude;
        VoteState {
            turnout: new_turnout,
            ..self.clone()
        }
    }
}

impl<
        Signal: Parameter
            + PartialOrd
            + Default
            + Copy
            + From<u32>
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
            + From<u32>
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
            + From<u32>
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
