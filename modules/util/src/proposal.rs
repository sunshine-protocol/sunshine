use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;

// associated with every proposal
pub type ProposalId = u32;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
#[non_exhaustive]
/// Proposal stages
pub enum ProposalStage {
    /// Applied but not sponsored
    Application,
    /// Sponsored and open to voting by members
    Voting,
    /// Passed but not executed
    Passed,
    /// Already executed
    Law,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Proposal for membership changes to the LLC
/// - join and levae types can both be represented in this form
pub struct MembershipProposal<AccountId, Shares, BalanceOf, BlockNumber> {
    /// The proposal's index
    pub proposal_id: ProposalId,
    /// The proposer's associated `AccountId`
    pub proposer: AccountId,
    /// The collateral promised and transferred from proposer to collective upon execution
    pub stake_promised: BalanceOf,
    /// The number of preferred shares requested to have or burn
    pub preferred_shares_requested: Shares,
    /// The number of common shares requested to have or burn
    pub common_shares_requested: Shares,
    /// The current stage of the proposal
    pub stage: ProposalStage,
    /// The block number at which the proposal was initially proposed
    pub time_proposed: BlockNumber,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Pay schedule for grant proposals
pub struct BasicPaySchedule<AccountId, BalanceOf, BlockNumber> {
    period_length: BlockNumber,
    payment_per_period: BalanceOf,
    recipient: AccountId,
    start_block: Option<BlockNumber>,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Proposal for membership changes to the LLC
/// - join and levae types can both be represented in this form
pub struct GrantProposal<AccountId, BalanceOf, BlockNumber> {
    /// The proposal's index
    pub proposal_id: ProposalId,
    /// The member that is sponsoring this proposal
    pub sponsor: AccountId,
    /// The amount expressed in the grant proposal
    pub pay_schedule: BasicPaySchedule<AccountId, BalanceOf, BlockNumber>,
    /// The current stage of the proposal
    pub stage: ProposalStage,
    /// The block number at which the proposal was initially proposed
    pub time_proposed: BlockNumber,
}
