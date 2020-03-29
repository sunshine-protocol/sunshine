use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;

pub type ProposalIndex = u32;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
#[non_exhaustive]
/// The proposal taxonomy
pub enum ProposalType {
    /// Proposal to join(/leave?) executive membership
    ExecutiveMembership,
    /// Proposal to add recipient group to the list of potential grantees
    GrantSpend,
}

impl Default for ProposalType {
    fn default() -> ProposalType {
        ProposalType::ExecutiveMembership
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// This has very little context
/// TODO: we can add another trait like `PollActiveProposal`, maybe requires it with more ContextfulOutcome
/// to get how close to threshold and how far along the progress is
pub enum SimplePollingOutcome<VoteId> {
    /// Moved from the current VoteId to a new VoteId; extra to add in other trait: required_thresholds, votes_left
    MovedToNextVote(VoteId, VoteId),
    /// The current VoteId stays the same
    StayedOnCurrentVote(VoteId),
    /// the proposal was approved (change ProposalStage)
    Approved,
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ProposalStage {
    /// Proposed by a member
    Proposed,
    /// Voting has started (should add more context)
    Voting,
    /// The proposal is approved but NOT executed
    Approved,
    /// The proposal has been executed (should add more context)
    Executed,
    /// The proposal has been tabled (rejected proposals are tabled for future reference)
    Tabled,
}

impl Default for ProposalStage {
    fn default() -> Self {
        ProposalStage::Proposed
    }
}

// -------------- OLD STUFF, might still be useful --------------

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Proposal for membership changes to the LLC
/// - join and levae types can both be represented in this form
pub struct MembershipProposal<AccountId, Shares, BalanceOf, BlockNumber> {
    /// The proposal's index
    pub proposal_id: ProposalIndex,
    /// The proposer's associated `AccountId`
    pub proposer: AccountId,
    /// The collateral promised and transferred from proposer to collective upon execution
    pub stake_promised: BalanceOf,
    /// The number of common shares requested to have or burn
    pub shares_requested: Shares,
    /// The current stage of the proposal
    pub stage: ProposalStage,
    /// The block number at which the proposal was initially proposed
    pub time_proposed: BlockNumber,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Pay schedule for grant proposals
pub struct BasicPaySchedule<AccountId, BalanceOf, BlockNumber> {
    period_length: BlockNumber,
    payment_per_period: BalanceOf,
    recipient: AccountId,
    start_block: Option<BlockNumber>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Proposal for membership changes to the LLC
/// - join and levae types can both be represented in this form
pub struct GrantProposal<AccountId, BalanceOf, BlockNumber> {
    /// The proposal's index
    pub proposal_id: ProposalIndex,
    /// The member that is sponsoring this proposal
    pub sponsor: AccountId,
    /// The amount expressed in the grant proposal
    pub pay_schedule: BasicPaySchedule<AccountId, BalanceOf, BlockNumber>,
    /// The current stage of the proposal
    pub stage: ProposalStage,
    /// The block number at which the proposal was initially proposed
    pub time_proposed: BlockNumber,
}
