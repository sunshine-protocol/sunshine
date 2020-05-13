use crate::{
    organization::{FormedOrganization, ShareID},
    share::SimpleShareGenesis,
    traits::{AccessGenesis, DepositWithdrawalOps, GetBalance, VerifyOwnership},
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
// TODO: add more information about the context INNER vs OUTER and how to organize these things
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
pub struct BankVault<AccountId, Currency> {
    // free capital, available for spends
    free: Currency,
    // set aside for future spends, already allocated but can be liquidated after free == 0?
    reserved: Currency,
    withdraw_permissions: Option<WithdrawalPermissions<AccountId>>,
}

impl<AccountId: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    BankVault<AccountId, Currency>
{
    pub fn free_funds(&self) -> Currency {
        self.free.clone()
    }
    pub fn reserved_funds(&self) -> Currency {
        self.reserved.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the state for an OnChainBankId, associated with each bank registered in the runtime
/// RULE: inner.free + inner.reserved + outer.free + outer.reserved == total balance in account
pub struct BankState<AccountId, Currency> {
    /// Registered organization identifier
    registered_org: u32,
    /// Vault available for inner shares
    inner: BankVault<AccountId, Currency>,
    /// Vault available for outer shares
    outer: BankVault<AccountId, Currency>,
}

impl<AccountId: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    BankState<AccountId, Currency>
{
    pub fn registered_org(&self) -> u32 {
        self.registered_org
    }
    pub fn inner_free_funds(&self) -> Currency {
        self.inner.free_funds()
    }
    pub fn inner_reserved_funds(&self) -> Currency {
        self.inner.reserved_funds()
    }
    pub fn outer_free_funds(&self) -> Currency {
        self.outer.free_funds()
    }
    pub fn outer_reserved_funds(&self) -> Currency {
        self.outer.reserved_funds()
    }
}
// TODO:
// ReWrite DepositWithdrawalOps to require attaching something that indicates more specific permissions
// delete GetBalance or replace
// delete VerifyOwnership or replace

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
