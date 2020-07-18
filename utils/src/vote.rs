use crate::traits::{
    Apply,
    Approved,
    Rejected,
    VoteVector,
};
use codec::{
    Decode,
    Encode,
};
use frame_support::Parameter;
use sp_std::prelude::*;

#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
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

impl Default for VoterView {
    fn default() -> VoterView {
        VoterView::NoVote
    }
}

#[derive(
    new, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
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

impl<Signal: Copy, Hash: Clone> VoteVector<Signal, VoterView, Hash>
    for Vote<Signal, Hash>
{
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of an ongoing vote
pub struct VoteState<Signal, BlockNumber, Hash> {
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
    passage_threshold: Signal,
    /// The threshold requirement for rejection
    rejection_threshold: Option<Signal>,
    /// The time at which this vote state is initialized
    initialized: BlockNumber,
    /// The time at which this vote state expires
    expires: Option<BlockNumber>,
    /// The vote outcome
    outcome: VoteOutcome,
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Default for VoteState<Signal, BlockNumber, Hash>
{
    fn default() -> VoteState<Signal, BlockNumber, Hash> {
        VoteState {
            topic: None,
            in_favor: 0u32.into(),
            against: 0u32.into(),
            turnout: 0u32.into(),
            all_possible_turnout: 0u32.into(),
            passage_threshold: 0u32.into(),
            rejection_threshold: None,
            initialized: BlockNumber::default(),
            expires: None,
            outcome: VoteOutcome::default(),
        }
    }
}

impl<
        Signal: Parameter
            + Copy
            + From<u32>
            + Default
            + sp_std::ops::Add<Output = Signal>
            + sp_std::ops::Sub<Output = Signal>
            + PartialOrd,
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > VoteState<Signal, BlockNumber, Hash>
{
    pub fn new(
        topic: Option<Hash>,
        all_possible_turnout: Signal,
        passage_threshold: Signal,
        rejection_threshold: Option<Signal>,
        initialized: BlockNumber,
        expires: Option<BlockNumber>,
    ) -> VoteState<Signal, BlockNumber, Hash> {
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
    ) -> VoteState<Signal, BlockNumber, Hash> {
        VoteState {
            topic,
            all_possible_turnout,
            passage_threshold: all_possible_turnout,
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
    pub fn passage_threshold(&self) -> Signal {
        self.passage_threshold
    }
    pub fn rejection_threshold(&self) -> Option<Signal> {
        self.rejection_threshold
    }
    pub fn outcome(&self) -> VoteOutcome {
        self.outcome
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
    fn set_outcome(&self) -> Self {
        let rejected = if let Some(rejection_outcome) = self.rejected() {
            rejection_outcome
        } else {
            false
        };
        if self.approved() {
            VoteState {
                outcome: VoteOutcome::Approved,
                ..self.clone()
            }
        } else if rejected {
            VoteState {
                outcome: VoteOutcome::Rejected,
                ..self.clone()
            }
        } else {
            self.clone()
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
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Approved for VoteState<Signal, BlockNumber, Hash>
{
    fn approved(&self) -> bool {
        self.in_favor() >= self.passage_threshold()
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
        BlockNumber: Parameter + Copy + Default,
        Hash: Clone,
    > Rejected for VoteState<Signal, BlockNumber, Hash>
{
    fn rejected(&self) -> Option<bool> {
        if let Some(rejection_threshold_set) = self.rejection_threshold() {
            Some(self.against() >= rejection_threshold_set)
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
            + sp_std::ops::Sub<Output = Signal>
            + PartialOrd,
        Hash: Clone,
        BlockNumber: Parameter + Copy + Default,
    > Apply<Signal, VoterView> for VoteState<Signal, BlockNumber, Hash>
{
    fn apply(
        &self,
        magnitude: Signal,
        old_direction: VoterView,
        new_direction: VoterView,
    ) -> Option<VoteState<Signal, BlockNumber, Hash>> {
        match (old_direction, new_direction) {
            (VoterView::NoVote, VoterView::InFavor) => {
                let new_turnout = self.turnout() + magnitude;
                let new_in_favor = self.in_favor() + magnitude;
                let new_vote_state = VoteState {
                    in_favor: new_in_favor,
                    turnout: new_turnout,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::NoVote, VoterView::Against) => {
                let new_turnout = self.turnout() + magnitude;
                let new_against = self.against() + magnitude;
                let new_vote_state = VoteState {
                    against: new_against,
                    turnout: new_turnout,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::NoVote, VoterView::Abstain) => {
                let new_turnout = self.turnout() + magnitude;
                let new_vote_state = VoteState {
                    turnout: new_turnout,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::InFavor, VoterView::Against) => {
                let new_in_favor = self.in_favor() - magnitude;
                let new_against = self.against() + magnitude;
                let new_vote_state = VoteState {
                    in_favor: new_in_favor,
                    against: new_against,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::InFavor, VoterView::Abstain) => {
                let new_in_favor = self.in_favor() - magnitude;
                let new_vote_state = VoteState {
                    in_favor: new_in_favor,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::Against, VoterView::InFavor) => {
                let new_against = self.against() - magnitude;
                let new_in_favor = self.in_favor() + magnitude;
                let new_vote_state = VoteState {
                    in_favor: new_in_favor,
                    against: new_against,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::Against, VoterView::Abstain) => {
                let new_against = self.against() - magnitude;
                let new_vote_state = VoteState {
                    against: new_against,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::Abstain, VoterView::InFavor) => {
                let new_in_favor = self.in_favor() + magnitude;
                let new_vote_state = VoteState {
                    in_favor: new_in_favor,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            (VoterView::Abstain, VoterView::Against) => {
                let new_against = self.against() + magnitude;
                let new_vote_state = VoteState {
                    against: new_against,
                    ..self.clone()
                };
                Some(new_vote_state.set_outcome())
            }
            // either no changes or not a supported vote change
            _ => None,
        }
    }
}

#[derive(
    PartialEq, Eq, Copy, Clone, Encode, Decode, sp_runtime::RuntimeDebug,
)]
#[non_exhaustive]
/// The vote's state and outcome
pub enum VoteOutcome {
    /// The VoteId in question has been reserved but is not yet open for voting (context is schedule)
    NotStarted,
    /// The VoteState associated with the VoteId is open to voting by the given `ShareId`
    Voting,
    /// The VoteState is approved
    Approved,
    /// The VoteState is rejected
    Rejected,
}

impl Default for VoteOutcome {
    fn default() -> Self {
        VoteOutcome::NotStarted
    }
}
