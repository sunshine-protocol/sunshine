use crate::traits::{
    Apply, Approved, ConsistentThresholdStructure, DeriveThresholdRequirement, Rejected, Revert,
    VoteVector,
};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::PerThing;
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The voter options (direction)
pub enum VoterView {
    /// Not yet voted
    NoVote,
    /// Voted in favor
    InFavor,
    /// Voted against
    Against,
    /// Acknowledged but abstained
    Abstain,
}

#[derive(new, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// Binary vote to express for/against with magnitude
/// ~ vectors have direction and magnitude, not to be confused with `Vec`
pub struct Vote<Signal, Hash> {
    magnitude: Signal,
    direction: VoterView,
    justification: Option<Hash>,
}

impl<Signal: Copy, Hash: Clone> Vote<Signal, Hash> {
    pub fn set_new_view(
        &self,
        new_direction: VoterView,
        new_justification: Option<Hash>,
    ) -> Option<Self> {
        if self.direction == new_direction {
            // new view not set because same object
            None
        } else {
            Some(Vote {
                magnitude: self.magnitude,
                direction: new_direction,
                justification: new_justification,
            })
        }
    }
}

impl<Signal: Copy, Hash: Clone> VoteVector<Signal, VoterView, Hash> for Vote<Signal, Hash> {
    fn magnitude(&self) -> Signal {
        self.magnitude
    }
    fn direction(&self) -> VoterView {
        self.direction
    }
    fn justification(&self) -> Option<Hash> {
        self.justification.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The type for the threshold
pub enum ThresholdType<Signal, FineArithmetic>
where
    FineArithmetic: PerThing,
{
    Count(Signal),
    Percentage(FineArithmetic),
}

impl<Signal: From<u32>, FineArithmetic: PerThing> Default
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
    Signal: From<u32>,
    FineArithmetic: PerThing,
{
    /// Support threshold
    support_required: ThresholdType<Signal, FineArithmetic>,
    /// Required turnout
    turnout_required: ThresholdType<Signal, FineArithmetic>,
}
impl<Signal: From<u32>, FineArithmetic: PerThing> From<(u32, u32)>
    for ThresholdConfig<Signal, FineArithmetic>
{
    fn from(other: (u32, u32)) -> ThresholdConfig<Signal, FineArithmetic> {
        ThresholdConfig {
            support_required: ThresholdType::Count(other.0.into()),
            turnout_required: ThresholdType::Count(other.1.into()),
        }
    }
}

impl<Signal: From<u32>, FineArithmetic: PerThing> ThresholdConfig<Signal, FineArithmetic> {
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

impl<Signal: From<u32> + Copy, FineArithmetic: PerThing + Copy> ConsistentThresholdStructure
    for ThresholdConfig<Signal, FineArithmetic>
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
        Signal: From<u32> + Copy,
        FineArithmetic: PerThing + sp_std::ops::Mul<Signal, Output = Signal>,
    > DeriveThresholdRequirement<Signal> for ThresholdConfig<Signal, FineArithmetic>
{
    fn derive_threshold_requirement(&self, turnout: Signal) -> Signal {
        match self.support_required {
            ThresholdType::Count(signal) => signal,
            ThresholdType::Percentage(signal) => signal * turnout,
        }
    }
    fn derive_turnout_requirement(&self, turnout: Signal) -> Signal {
        match self.turnout_required {
            ThresholdType::Count(signal) => signal,
            ThresholdType::Percentage(signal) => signal * turnout,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of an ongoing vote
pub struct VoteState<Signal, FineArithmetic, BlockNumber, Hash>
where
    Signal: From<u32>,
    FineArithmetic: PerThing,
{
    /// Vote state must often be anchored to offchain state, cid
    topic: Option<Hash>,
    /// All signal in favor
    in_favor: Signal,
    /// All signal against
    against: Signal,
    /// All signal that votes at all
    turnout: Signal,
    /// All signal that can vote
    all_possible_turnout: Signal,
    /// The threshold requirement for passage
    passage_threshold: ThresholdConfig<Signal, FineArithmetic>,
    /// The threshold requirement for rejection
    rejection_threshold: Option<ThresholdConfig<Signal, FineArithmetic>>,
    /// The time at which this vote state is initialized
    initialized: BlockNumber,
    /// The time at which this vote state expires
    expires: Option<BlockNumber>,
    /// The vote outcome
    outcome: VoteOutcome,
}

impl<
        Signal: Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Default for VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    fn default() -> VoteState<Signal, FineArithmetic, BlockNumber, Hash> {
        VoteState {
            topic: None,
            in_favor: 0u32.into(),
            against: 0u32.into(),
            turnout: 0u32.into(),
            all_possible_turnout: 0u32.into(),
            passage_threshold: ThresholdConfig::default(),
            rejection_threshold: None,
            initialized: BlockNumber::default(),
            expires: None,
            outcome: VoteOutcome::default(),
        }
    }
}

impl<
        Signal: Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    pub fn new(
        topic: Option<Hash>,
        all_possible_turnout: Signal,
        passage_threshold: ThresholdConfig<Signal, FineArithmetic>,
        rejection_threshold: Option<ThresholdConfig<Signal, FineArithmetic>>,
        initialized: BlockNumber,
        expires: Option<BlockNumber>,
    ) -> VoteState<Signal, FineArithmetic, BlockNumber, Hash> {
        VoteState {
            topic,
            all_possible_turnout,
            passage_threshold,
            rejection_threshold,
            initialized,
            expires,
            outcome: VoteOutcome::Voting,
            ..Default::default()
        }
    }
    pub fn new_unanimous_consent(
        topic: Option<Hash>,
        all_possible_turnout: Signal,
        initialized: BlockNumber,
        expires: Option<BlockNumber>,
    ) -> VoteState<Signal, FineArithmetic, BlockNumber, Hash> {
        let unanimous_passage_threshold =
            ThresholdConfig::new_signal_count_threshold(all_possible_turnout, all_possible_turnout);
        VoteState {
            topic,
            all_possible_turnout,
            passage_threshold: unanimous_passage_threshold,
            rejection_threshold: None,
            initialized,
            expires,
            outcome: VoteOutcome::Voting,
            ..Default::default()
        }
    }
    pub fn topic(&self) -> Option<Hash> {
        self.topic.clone()
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
    pub fn passage_threshold(&self) -> ThresholdConfig<Signal, FineArithmetic> {
        self.passage_threshold
    }
    pub fn rejection_threshold(&self) -> Option<ThresholdConfig<Signal, FineArithmetic>> {
        self.rejection_threshold
    }
    pub fn outcome(&self) -> VoteOutcome {
        self.outcome
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
    pub fn set_outcome(&self, new_outcome: VoteOutcome) -> Self {
        VoteState {
            outcome: new_outcome,
            ..self.clone()
        }
    }
    pub fn update_topic_and_clear_state(&self, new_topic: Hash) -> Self {
        VoteState {
            in_favor: 0u32.into(),
            against: 0u32.into(),
            turnout: 0u32.into(),
            topic: Some(new_topic),
            ..self.clone()
        }
    }
    pub fn update_topic_without_clearing_state(&self, new_topic: Hash) -> Self {
        VoteState {
            topic: Some(new_topic),
            ..self.clone()
        }
    }
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + PartialOrd
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        FineArithmetic: PerThing + Default + sp_std::ops::Mul<Signal, Output = Signal>,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Approved for VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    fn approved(&self) -> bool {
        self.in_favor()
            > self
                .passage_threshold
                .derive_threshold_requirement(self.all_possible_turnout())
            && self.turnout()
                > self
                    .passage_threshold
                    .derive_turnout_requirement(self.all_possible_turnout())
    }
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + PartialOrd
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        FineArithmetic: PerThing + Default + sp_std::ops::Mul<Signal, Output = Signal>,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Rejected for VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    fn rejected(&self) -> Option<bool> {
        if let Some(rejection_threshold_set) = self.rejection_threshold() {
            Some(
                self.against()
                    > rejection_threshold_set
                        .derive_threshold_requirement(self.all_possible_turnout())
                    && self.turnout()
                        > rejection_threshold_set
                            .derive_turnout_requirement(self.all_possible_turnout()),
            )
        } else {
            // rejection threshold not set!
            None
        }
    }
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        Hash: Clone,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > Apply<Vote<Signal, Hash>> for VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    fn apply(
        &self,
        vote: Vote<Signal, Hash>,
    ) -> VoteState<Signal, FineArithmetic, BlockNumber, Hash> {
        match vote.direction() {
            VoterView::InFavor => self.add_in_favor_vote(vote.magnitude()),
            VoterView::Against => self.add_against_vote(vote.magnitude()),
            VoterView::Abstain => self.add_abstention(vote.magnitude()),
            _ => self.clone(),
        }
    }
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        Hash: Clone,
        FineArithmetic: PerThing + Default,
        BlockNumber: Parameter + Copy + Default,
    > Revert<Vote<Signal, Hash>> for VoteState<Signal, FineArithmetic, BlockNumber, Hash>
{
    fn revert(
        &self,
        vote: Vote<Signal, Hash>,
    ) -> VoteState<Signal, FineArithmetic, BlockNumber, Hash> {
        match vote.direction() {
            VoterView::InFavor => self.remove_in_favor_vote(vote.magnitude()),
            VoterView::Against => self.remove_against_vote(vote.magnitude()),
            VoterView::Abstain => self.remove_abstention(vote.magnitude()),
            _ => self.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
#[non_exhaustive]
/// The vote's state and outcome
pub enum VoteOutcome {
    /// The VoteId in question has been reserved but is not yet open for voting (context is schedule)
    NotStarted,
    /// The VoteState associated with the VoteId is open to voting by the given `ShareId`
    Voting,
    /// The VoteState is approved, thereby unlocking the next `VoteId` if it wraps Some(VoteId)
    ApprovedAndNotExpired,
    /// Past the expiry time and approved
    ApprovedAndExpired,
    /// The VoteState is rejected and all dependent `VoteId`s are not opened
    Rejected,
}

impl Default for VoteOutcome {
    fn default() -> Self {
        VoteOutcome::NotStarted
    }
}
