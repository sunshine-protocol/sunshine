use codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
#[non_exhaustive]
/// The proposal type taxonomy
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
/// The state of a proposal made in [`bank`](../../bank/index.html)
pub enum ProposalStage {
    /// Proposed by a member
    Proposed,
    /// Voting has started
    Voting,
    /// The proposal is approved but NOT executed
    Approved,
    /// The proposal has been executed
    Executed,
    /// The proposal has been tabled (rejected proposals are tabled for future reference)
    Tabled,
}

impl Default for ProposalStage {
    fn default() -> Self {
        ProposalStage::Proposed
    }
}
