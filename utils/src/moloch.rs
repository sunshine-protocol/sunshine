use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum ProposalState<VoteId> {
    WaitingForApproval,
    Voting(VoteId),
    ApprovedButNotExecuted,
    ApprovedAndExecuted,
}

#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, RuntimeDebug)]
pub struct MembershipProposal<
    BankId,
    PropId,
    Currency,
    Shares,
    AccountId,
    State,
> {
    id: (BankId, PropId),
    tribute: Currency,
    shares_requested: Shares,
    applicant: AccountId,
    state: State,
}

impl<
        BankId: Copy,
        PropId: Copy,
        Currency: Copy,
        Shares: Copy,
        AccountId: Clone,
        VoteId: Copy,
    >
    MembershipProposal<
        BankId,
        PropId,
        Currency,
        Shares,
        AccountId,
        ProposalState<VoteId>,
    >
{
    pub fn new(
        bank_id: BankId,
        prop_id: PropId,
        tribute: Currency,
        shares_requested: Shares,
        applicant: AccountId,
    ) -> Self {
        Self {
            id: (bank_id, prop_id),
            tribute,
            shares_requested,
            applicant,
            state: ProposalState::WaitingForApproval,
        }
    }
    pub fn bank_id(&self) -> BankId {
        self.id.0
    }
    pub fn prop_id(&self) -> PropId {
        self.id.1
    }
    pub fn tribute(&self) -> Currency {
        self.tribute
    }
    pub fn shares_requested(&self) -> Shares {
        self.shares_requested
    }
    pub fn applicant(&self) -> AccountId {
        self.applicant.clone()
    }
    pub fn state(&self) -> ProposalState<VoteId> {
        self.state
    }
    pub fn set_state(&self, state: ProposalState<VoteId>) -> Self {
        Self {
            state,
            ..self.clone()
        }
    }
}
