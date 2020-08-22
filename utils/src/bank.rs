use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

#[derive(
    new, PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct BankState<
    BankId,
    AccountId,
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ThresholdId,
> {
    id: BankId,
    // Registered organization identifier
    org: OrgId,
    // Layered sudo, selection should eventually be revocable by the group
    controller: Option<AccountId>,
    // identifier for registered vote threshold
    threshold_id: ThresholdId,
}

impl<
        BankId: Copy,
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ThresholdId: Copy,
    > BankState<BankId, AccountId, OrgId, ThresholdId>
{
    pub fn id(&self) -> BankId {
        self.id
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn controller(&self) -> Option<AccountId> {
        self.controller.clone()
    }
    pub fn threshold_id(&self) -> ThresholdId {
        self.threshold_id
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
pub struct SpendProposal<BankId, SpendId, Currency, AccountId, State> {
    id: (BankId, SpendId),
    amount: Currency,
    dest: AccountId,
    state: State,
}

impl<
        BankId: Copy,
        SpendId: Copy,
        Currency: Copy,
        AccountId: Clone,
        VoteId: Copy,
    > SpendProposal<BankId, SpendId, Currency, AccountId, SpendState<VoteId>>
{
    pub fn new(
        bank_id: BankId,
        spend_id: SpendId,
        amount: Currency,
        dest: AccountId,
    ) -> Self {
        Self {
            id: (bank_id, spend_id),
            amount,
            dest,
            state: SpendState::WaitingForApproval,
        }
    }
    pub fn bank_id(&self) -> BankId {
        self.id.0
    }
    pub fn spend_id(&self) -> SpendId {
        self.id.1
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
