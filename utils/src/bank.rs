use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

#[derive(
    new, Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct BankSpend<BankId, SpendId> {
    pub bank: BankId,
    pub spend: SpendId,
}

#[derive(
    new, PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct BankState<
    AccountId,
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
> {
    // Registered organization identifier
    org: OrgId,
    // Layered sudo, selection should eventually be revocable by the group
    controller: Option<AccountId>,
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    > BankState<AccountId, OrgId>
{
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn controller(&self) -> Option<AccountId> {
        self.controller.clone()
    }
    pub fn is_org(&self, org: OrgId) -> bool {
        org == self.org()
    }
    pub fn is_controller(&self, purported_sudo: &AccountId) -> bool {
        if let Some(op) = self.controller() {
            &op == purported_sudo
        } else {
            false
        }
    }
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum SpendState<VoteId> {
    WaitingForApproval,
    Voting(VoteId),
    ApprovedButNotExecuted,
    ApprovedAndExecuted,
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct SpendProposal<Currency, AccountId, State> {
    amount: Currency,
    dest: AccountId,
    state: State,
}

impl<Currency: Copy, AccountId: Clone, VoteId: Copy>
    SpendProposal<Currency, AccountId, SpendState<VoteId>>
{
    pub fn new(amount: Currency, dest: AccountId) -> Self {
        Self {
            amount,
            dest,
            state: SpendState::WaitingForApproval,
        }
    }
    pub fn amount(&self) -> Currency {
        self.amount
    }
    pub fn dest(&self) -> AccountId {
        self.dest.clone()
    }
    pub fn state(&self) -> SpendState<VoteId> {
        self.state
    }
    pub fn set_state(&self, state: SpendState<VoteId>) -> Self {
        Self {
            state,
            ..self.clone()
        }
    }
}
