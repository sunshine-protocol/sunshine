use crate::{proposal::ProposalType, traits::Approved};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::PerThing;

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

    // TODO: get this to work later and remove the `pub` parameters on the `ThresholdConfig` struct
    // pub fn configure_vote_threshold<Signal: Parameter>(&self, possible_turnout: Signal) -> (Signal, Signal) {
    //     let first = self.passage_threshold_pct * possible_turnout;
    //     let second = self.turnout_threshold_pct * possible_turnout;
    //     (first, second)
    // }
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
pub struct VoteState<ShareId, VoteId, Signal, BlockNumber> {
    /// The only share_id that can vote in this VoteState
    pub electorate: ShareId,
    /// The identifier associated with this `VoteState` (for managing associated state ie `VoterStatus`)
    pub vote_id: VoteId,
    /// Signal in favor
    pub in_favor: Signal,
    /// Signal against
    pub against: Signal,
    /// All signal that votes
    pub turnout: Signal,
    /// The type of proposal (decides vote algo)
    pub proposal_type: ProposalType,
    /// The threshold for passage
    pub threshold: VoteThreshold<Signal, BlockNumber>,
    /// The time at which this is initialized (4_TTL_C_l8r)
    pub initialized: BlockNumber,
    /// The time at which this vote state expired
    pub expires: BlockNumber,
}

impl<
        ShareId: Parameter,
        VoteId: Parameter,
        Signal: Parameter + PartialOrd,
        BlockNumber: Parameter,
    > Approved for VoteState<ShareId, VoteId, Signal, BlockNumber>
{
    fn approved(&self) -> bool {
        self.in_favor > self.threshold.get_passage_threshold()
            && self.turnout > self.threshold.get_turnout_threshold()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
#[non_exhaustive]
/// The possible outcomes for apply_vote
/// - should be associated with runtime hooks
pub enum Outcome {
    Voting,
    Approved,
    Rejected,
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::Voting
    }
}
