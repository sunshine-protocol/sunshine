use crate::traits::Increment;
use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_core::TypeId;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

/// An on-chain treasury identifier, exactly like `ModuleId`
#[derive(
    Clone,
    Copy,
    Eq,
    PartialEq,
    Default,
    Encode,
    Decode,
    sp_runtime::RuntimeDebug,
)]
pub struct OnChainTreasuryID(pub [u8; 8]);

impl Increment for OnChainTreasuryID {
    fn increment(self) -> OnChainTreasuryID {
        let old_inner = u64::from_be_bytes(self.0);
        let new_inner = old_inner.saturating_add(1u64);
        OnChainTreasuryID(new_inner.to_be_bytes())
    }
}

impl TypeId for OnChainTreasuryID {
    const TYPE_ID: [u8; 4] = *b"bank";
}

#[derive(
    new, Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct BankSpend<BankId, SpendId> {
    pub bank: BankId,
    pub spend: SpendId,
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum BankOrAccount<BankId, AccountId> {
    Bank(BankId),
    Account(AccountId),
}

impl<OnChainTreasuryID, AccountId: Clone>
    BankOrAccount<OnChainTreasuryID, AccountId>
{
    pub fn bank_id(self) -> Option<OnChainTreasuryID> {
        match self {
            BankOrAccount::Bank(id) => Some(id),
            BankOrAccount::Account(_) => None,
        }
    }
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
