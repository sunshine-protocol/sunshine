use crate::{
    organization::ShareID,
    share::SimpleShareGenesis,
    traits::{AccessGenesis, DepositWithdrawalOps},
};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_core::TypeId;
use sp_runtime::traits::{AtLeast32Bit, Member, Zero};
use sp_runtime::{PerThing, Permill};
use sp_std::{marker::PhantomData, prelude::*};

/// An off-chain treasury id
pub type OffChainTreasuryID = u32;

/// An on-chain treasury identifier, exactly like `ModuleId`
#[derive(Clone, Copy, Eq, PartialEq, Default, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct OnChainTreasuryID(pub [u8; 8]);

impl OnChainTreasuryID {
    pub fn iterate(&self) -> OnChainTreasuryID {
        let old_inner = u64::from_be_bytes(self.0);
        let new_inner = old_inner.saturating_add(1u64);
        OnChainTreasuryID(new_inner.to_be_bytes())
    }
}

impl TypeId for OnChainTreasuryID {
    const TYPE_ID: [u8; 4] = *b"bank";
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct DepositInfo<AccountId, Hash, Currency, FineArithmetic> {
    // for uniqueness
    salt: u32,
    depositer: AccountId,
    // TODO: change this to an enum, it might wrap an IpfsReference (which is what this hash was for anyway)
    reason: Hash,
    amount: Currency,
    // default None is no capital reserved for savings
    // TODO: add configurable API for bank's enforcing savings on deposits with certain reasons
    savings_pct: Option<FineArithmetic>,
}
impl<AccountId: Clone, Hash: Clone, Currency: Clone, FineArithmetic: PerThing>
    DepositInfo<AccountId, Hash, Currency, FineArithmetic>
{
    pub fn new(
        salt: u32,
        depositer: AccountId,
        reason: Hash,
        amount: Currency,
        savings_pct: Option<FineArithmetic>,
    ) -> DepositInfo<AccountId, Hash, Currency, FineArithmetic> {
        DepositInfo {
            salt,
            depositer,
            reason,
            amount,
            savings_pct,
        }
    }
    pub fn depositer(&self) -> AccountId {
        self.depositer.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn savings_pct(&self) -> Option<FineArithmetic> {
        self.savings_pct
    }
    // TODO: make this take &mut instead? is changing it better than all these inner clones?
    pub fn iterate_salt(&self) -> Self {
        DepositInfo {
            salt: self.salt + 1u32,
            depositer: self.depositer.clone(),
            reason: self.reason.clone(),
            amount: self.amount.clone(),
            savings_pct: self.savings_pct,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the state for an OnChainBankId, associated with each bank registered in the runtime
/// INVARIANT: savings + reserved_for_grant_payouts = T::Currency::total_balance(Self::account_id(OnChainBankId))
pub struct BankState<AccountId, Currency> {
    /// Reserved only for liquidation upon burning shares OR vote-based spending by the collective
    savings: Currency,
    /// Reserved only for withdrawal by grant team, some portion might be moved to savings if the group prefers this structure
    reserved_for_spends: Currency,
    /// Permissions for making withdrawals
    permissions: WithdrawalPermissions<AccountId>,
}

impl<AccountId: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    BankState<AccountId, Currency>
{
    pub fn savings(&self) -> Currency {
        self.savings.clone()
    }
    pub fn reserved_for_spends(&self) -> Currency {
        self.reserved_for_spends.clone()
    }
    pub fn permissions(&self) -> WithdrawalPermissions<AccountId> {
        self.permissions.clone()
    }
    // getter helper to verify permissions for target groups
    pub fn verify_sudo(&self, account: &AccountId) -> bool {
        match &self.permissions {
            WithdrawalPermissions::Sudo(acc) => acc == account,
            _ => false,
        }
    }
    pub fn extract_weighted_share_group_id(&self) -> Option<(u32, u32)> {
        match &self.permissions {
            WithdrawalPermissions::RegisteredShareGroup(org_id, wrapped_share_id) => {
                match wrapped_share_id {
                    ShareID::WeightedAtomic(share_id) => Some((*org_id, *share_id)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    pub fn init(
        beginning_balance: Currency,
        pct_reserved_for_spends: Option<Permill>,
        permissions: WithdrawalPermissions<AccountId>,
    ) -> BankState<AccountId, Currency> {
        let reserved_for_spends = if let Some(pct) = pct_reserved_for_spends {
            pct * beginning_balance.clone()
        } else {
            Currency::zero()
        };
        let savings = beginning_balance - reserved_for_spends;
        BankState {
            savings,
            reserved_for_spends: Currency::zero(),
            permissions,
        }
    }
}

impl<AccountId: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    DepositWithdrawalOps<Currency, Permill> for BankState<AccountId, Currency>
{
    fn apply_deposit(
        &self,
        amount: Currency,
        reserved_for_savings: Option<Permill>,
    ) -> BankState<AccountId, Currency> {
        let increase_in_savings = if let Some(pctage_to_save) = reserved_for_savings {
            pctage_to_save * amount.clone()
        } else {
            Currency::zero()
        };
        let increase_in_reserved_for_grant_payouts = amount - increase_in_savings.clone();
        let savings = self.savings() + increase_in_savings;
        let reserved_for_spends =
            self.reserved_for_spends() + increase_in_reserved_for_grant_payouts;
        let permissions = self.permissions();
        BankState {
            savings,
            reserved_for_spends,
            permissions,
        }
    }
    fn spend_from_total(&self, amount: Currency) -> Option<Self> {
        let total_accessible_capital_for_sudo =
            self.savings.clone() + self.reserved_for_spends.clone();
        if amount <= total_accessible_capital_for_sudo {
            let new_balance = if let Some(new_savings) = self.savings.clone().checked_sub(&amount) {
                (new_savings, self.reserved_for_spends.clone())
            } else {
                let amount_to_withdraw_from_reserved = amount - self.savings.clone();
                let new_reserved_for_spends =
                    self.reserved_for_spends.clone() - amount_to_withdraw_from_reserved;
                (Currency::zero(), new_reserved_for_spends)
            };
            Some(BankState {
                savings: new_balance.0,
                reserved_for_spends: new_balance.1,
                permissions: self.permissions.clone(),
            })
        } else {
            None
        }
    }
    fn spend_from_reserved_spends(&self, amount: Currency) -> Option<Self> {
        if amount <= self.reserved_for_spends.clone() {
            let new_reserved_for_spends = self.reserved_for_spends.clone() - amount;
            Some(BankState {
                savings: self.savings.clone(),
                reserved_for_spends: new_reserved_for_spends,
                permissions: self.permissions.clone(),
            })
        } else {
            None
        }
    }
    fn spend_from_savings(&self, amount: Currency) -> Option<Self> {
        if amount <= self.savings.clone() {
            let new_savings = self.savings.clone() - amount;
            Some(BankState {
                savings: new_savings,
                reserved_for_spends: self.reserved_for_spends.clone(),
                permissions: self.permissions.clone(),
            })
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub enum WithdrawalPermissions<AccountId> {
    // a single account controls withdrawal permissions, can approve on-chain requests
    Sudo(AccountId),
    // flat group, implied equal withdrawal permissions \forall members
    RegisteredOrganizationFlatMembership(u32),
    // The members in a registered share group can withdraw as per their proportion of ownership
    // HAPPY PATH!
    RegisteredShareGroup(u32, ShareID),
}

impl<AccountId> Default for WithdrawalPermissions<AccountId> {
    fn default() -> WithdrawalPermissions<AccountId> {
        // this will be the address for ecosystem governance of taxes
        WithdrawalPermissions::RegisteredOrganizationFlatMembership(0u32)
    }
}

// 000experiment with type states000

// Withdrawal States as Zero Sized Types To Enforce State Guarantees
#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct RequestPendingReview;
#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct RequestApproved;
#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct RequestExecuted;

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct Spend<State, Hash, Currency> {
    reason: Hash,
    amount: Currency,
    // to mark valid state transitions, type states
    _marker: PhantomData<State>,
}

impl<Hash, Currency> Spend<RequestPendingReview, Hash, Currency> {
    pub fn new(reason: Hash, amount: Currency) -> Spend<RequestPendingReview, Hash, Currency> {
        Spend {
            reason,
            amount,
            _marker: PhantomData::<RequestPendingReview>,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// A wrapper around a vec to create specific impl From<Vec<AccountId>> for Vec<(AccountId, FineArithmetic)>
pub struct FlatGroup<AccountId>(Vec<AccountId>);

#[derive(Clone, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This will be useful for liquidating the bank with automatic rules...
pub struct PercentageOwnership<AccountId, FineArithmetic>(Vec<(AccountId, FineArithmetic)>);

impl<AccountId> From<FlatGroup<AccountId>> for PercentageOwnership<AccountId, Permill> {
    fn from(flat_group: FlatGroup<AccountId>) -> PercentageOwnership<AccountId, Permill> {
        let size = flat_group.0.len() as u32;
        let constant_ownership: Permill = Permill::from_rational_approximation(1u32, size);
        let ownership_structure = flat_group
            .0
            .into_iter()
            .map(|account| (account, constant_ownership))
            .collect::<Vec<(AccountId, Permill)>>();
        PercentageOwnership(ownership_structure)
    }
}

impl<
        AccountId: Clone,
        Shares: Copy + Clone + From<u32> + Parameter + Member + AtLeast32Bit + Codec,
    > From<SimpleShareGenesis<AccountId, Shares>> for PercentageOwnership<AccountId, Permill>
{
    fn from(
        weighted_group: SimpleShareGenesis<AccountId, Shares>,
    ) -> PercentageOwnership<AccountId, Permill> {
        let total = weighted_group.total();
        let ownership_structure = weighted_group
            .account_ownership()
            .into_iter()
            .map(|(account, shares)| {
                let ownership: Permill = Permill::from_rational_approximation(shares, total);
                (account, ownership)
            })
            .collect::<Vec<(AccountId, Permill)>>();
        PercentageOwnership(ownership_structure)
    }
}

// ~~ Off Chain Bank ~~

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
// TODO: add Currency type that is more compatible with off chain payments (USD)
pub struct Payment<AccountId, Currency> {
    salt: u32,
    sender: AccountId,
    receiver: AccountId,
    amount: Currency,
}

impl<AccountId: Clone, Currency: Clone> Payment<AccountId, Currency> {
    pub fn new(
        salt: u32,
        sender: AccountId,
        receiver: AccountId,
        amount: Currency,
    ) -> Payment<AccountId, Currency> {
        Payment {
            salt,
            sender,
            receiver,
            amount,
        }
    }
    pub fn salt(&self) -> u32 {
        self.salt
    }
    // TODO: make this take &mut instead? is changing it better than all these inner clones?
    pub fn iterate_salt(&self) -> Self {
        Payment {
            salt: self.salt + 1u32,
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            amount: self.amount.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct PaymentConfirmation {
    sender_claims: bool,
    recipient_confirms: bool,
}

impl PaymentConfirmation {
    pub fn from_sender_claims() -> Self {
        Self {
            sender_claims: true,
            recipient_confirms: false,
        }
    }
    pub fn put_recipient_confirms(&self) -> Self {
        Self {
            sender_claims: self.sender_claims,
            recipient_confirms: true,
        }
    }
    pub fn recipient_confirmation(&self) -> bool {
        self.recipient_confirms
    }
    pub fn total_confirmation(&self) -> bool {
        self.sender_claims && self.recipient_confirms
    }
}
