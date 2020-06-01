use crate::{
    organization::ShareID,
    share::SimpleShareGenesis,
    traits::{
        AccessGenesis, CommitSpendReservation, DepositSpendOps, FreeToReserved, GetBalance,
        MoveFundsOutCommittedOnly, MoveFundsOutUnCommittedOnly,
    },
};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_core::TypeId;
use sp_runtime::{
    traits::{AtLeast32Bit, Member, Zero},
    Permill,
};
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

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// All the other counter identifiers in this module that track state associated with bank account governance
pub enum BankMapID {
    Deposit,
    Reservation,
    InternalTransfer,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct BankTrackerIdentifier {
    treasury_id: OnChainTreasuryID,
    tracker_id: BankTrackerID,
}

impl BankTrackerIdentifier {
    pub fn new(treasury_id: OnChainTreasuryID, tracker_id: BankTrackerID) -> BankTrackerIdentifier {
        BankTrackerIdentifier {
            treasury_id,
            tracker_id,
        }
    }
}

impl From<(OnChainTreasuryID, BankTrackerID)> for BankTrackerIdentifier {
    fn from(other: (OnChainTreasuryID, BankTrackerID)) -> BankTrackerIdentifier {
        BankTrackerIdentifier::new(other.0, other.1)
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// Identifiers for tracking actions by specific individual AccountId to _eventually_ enforce limits on this behavior
pub enum BankTrackerID {
    // acceptable only if the withdrawer burns their ownership
    SpentFromFree,
    // allowed from any member of the bank's controller
    ReservedSpend,
    // wraos reservation_id, allowed from any member of bank's controller
    UnReservedSpendFromUnCommitted(u32),
    // wraps reservation_id, allowed from any member of reservation's controller
    UnReservedSpendFromCommitted(u32),
    // wraps reservation_id, allowed from any member of spend reservation's controller
    CommitSpend(u32),
    // wraps reservation_id, allowed from any member of the controller in the reference
    InternalTransferMade(u32),
    // wraps transfer_id, only acceptable withdrawal from reserved, must follow configured decision process
    SpentFromReserved(u32),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The simplest `GovernanceConfig`
/// - has no magnitude context and is limited for that reason
/// - future version will use revocable representative governance
pub enum WithdrawalPermissions<AccountId> {
    // two accounts can reserve free capital for spending
    AnyOfTwoAccounts(AccountId, AccountId),
    // any account in org
    AnyAccountInOrg(u32),
    // all accounts in this organization can reserve free capital for spending
    AnyMemberOfOrgShareGroup(u32, ShareID),
}

impl<AccountId> Default for WithdrawalPermissions<AccountId> {
    fn default() -> WithdrawalPermissions<AccountId> {
        WithdrawalPermissions::AnyAccountInOrg(0u32)
    }
}

impl<AccountId> WithdrawalPermissions<AccountId> {
    pub fn extract_org_weighted_share_id(&self) -> Option<(u32, u32)> {
        match self {
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, wrapped_share_id) => {
                match wrapped_share_id {
                    ShareID::WeightedAtomic(share_id) => Some((*org_id, *share_id)),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}
use crate::bounty::{ReviewBoard, TeamID};
impl<AccountId, Hash, WeightThreshold> From<ReviewBoard<AccountId, Hash, WeightThreshold>>
    for WithdrawalPermissions<AccountId>
{
    fn from(
        other: ReviewBoard<AccountId, Hash, WeightThreshold>,
    ) -> WithdrawalPermissions<AccountId> {
        match other {
            ReviewBoard::FlatPetitionReview(_, org_id, share_id, _, _, _) => {
                WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, ShareID::Flat(share_id))
            }
            ReviewBoard::WeightedThresholdReview(_, org_id, share_id, _, _) => {
                WithdrawalPermissions::AnyMemberOfOrgShareGroup(
                    org_id,
                    ShareID::WeightedAtomic(share_id),
                )
            }
        }
    }
}
impl<AccountId: Clone + PartialEq> From<TeamID<AccountId>> for WithdrawalPermissions<AccountId> {
    fn from(other: TeamID<AccountId>) -> WithdrawalPermissions<AccountId> {
        WithdrawalPermissions::AnyMemberOfOrgShareGroup(
            other.org(),
            ShareID::WeightedAtomic(other.weighted_share_id()),
        )
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the state for an OnChainBankId, associated with each bank registered in the runtime
pub struct BankState<GovernanceConfig, Currency> {
    /// Registered organization identifier
    registered_org: u32,
    // free capital, available for spends
    free: Currency,
    // set aside for future spends, already allocated but can only be _liquidated_ after free == 0?
    reserved: Currency,
    // owner of vault, likely WithdrawalPermissions<AccountId>
    owner_s: GovernanceConfig,
}

impl<GovernanceConfig: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    BankState<GovernanceConfig, Currency>
{
    pub fn new_from_deposit(
        registered_org: u32,
        amount: Currency,
        owner_s: GovernanceConfig,
    ) -> Self {
        BankState {
            registered_org,
            free: amount,
            reserved: Currency::zero(),
            owner_s,
        }
    }
    pub fn registered_org(&self) -> u32 {
        self.registered_org
    }
    pub fn free(&self) -> Currency {
        self.free.clone()
    }
    pub fn reserved(&self) -> Currency {
        self.reserved.clone()
    }
    pub fn owner_s(&self) -> GovernanceConfig {
        self.owner_s.clone()
    }
    pub fn is_owner_s(&self, cmp_owner: GovernanceConfig) -> bool {
        cmp_owner == self.owner_s
    }
}

impl<
        GovernanceConfig: Clone + PartialEq,
        Currency: Zero + AtLeast32Bit + Clone + sp_std::ops::Add + sp_std::ops::Sub,
    > FreeToReserved<Currency> for BankState<GovernanceConfig, Currency>
{
    fn move_from_free_to_reserved(&self, amount: Currency) -> Option<Self> {
        if self.free() >= amount {
            // safe because of above conditional
            let new_free = self.free() - amount.clone();
            let new_reserved = self.reserved() + amount;
            Some(BankState {
                registered_org: self.registered_org(),
                free: new_free,
                reserved: new_reserved,
                owner_s: self.owner_s(),
            })
        } else {
            // failed, not enough in free to make reservation of amount
            None
        }
    }
}

impl<
        GovernanceConfig: Clone + PartialEq,
        Currency: Zero + AtLeast32Bit + Clone + sp_std::ops::Add,
    > GetBalance<Currency> for BankState<GovernanceConfig, Currency>
{
    fn total_free_funds(&self) -> Currency {
        self.free()
    }
    fn total_reserved_funds(&self) -> Currency {
        self.reserved()
    }
    fn total_funds(&self) -> Currency {
        self.free() + self.reserved()
    }
}

impl<GovernanceConfig: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    DepositSpendOps<Currency> for BankState<GovernanceConfig, Currency>
{
    // infallible
    fn deposit_into_free(&self, amount: Currency) -> Self {
        let new_free = self.free() + amount;
        BankState {
            registered_org: self.registered_org(),
            free: new_free,
            reserved: self.reserved(),
            owner_s: self.owner_s(),
        }
    }
    fn deposit_into_reserved(&self, amount: Currency) -> Self {
        let new_reserved = self.reserved() + amount;
        BankState {
            registered_org: self.registered_org(),
            free: self.free(),
            reserved: new_reserved,
            owner_s: self.owner_s(),
        }
    }
    // fallible, not enough capital in relative account
    fn spend_from_free(&self, amount: Currency) -> Option<Self> {
        if self.free() >= amount {
            let new_free = self.free() - amount;
            Some(BankState {
                registered_org: self.registered_org(),
                free: new_free,
                reserved: self.reserved(),
                owner_s: self.owner_s(),
            })
        } else {
            // not enough capital in account, spend failed
            None
        }
    }
    fn spend_from_reserved(&self, amount: Currency) -> Option<Self> {
        if self.reserved() >= amount {
            let new_reserved = self.reserved() - amount;
            Some(BankState {
                registered_org: self.registered_org(),
                free: self.free(),
                reserved: new_reserved,
                owner_s: self.owner_s(),
            })
        } else {
            // not enough capital in account, spend failed
            None
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct CommitmentInfo<Hash, Currency> {
    reason: Hash,
    amount: Currency,
}

impl<Hash: Clone, Currency: Clone> CommitmentInfo<Hash, Currency> {
    pub fn new(reason: Hash, amount: Currency) -> CommitmentInfo<Hash, Currency> {
        CommitmentInfo { reason, amount }
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn reason(&self) -> Hash {
        self.reason.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct DepositInfo<AccountId, Hash, Currency> {
    // Depositer's identity
    depositer: AccountId,
    // Reason for the deposit, an ipfs reference
    reason: Hash,
    // Total amount of deposit into bank account (before fees, if any)
    amount: Currency,
} // TODO: attach an enum Tax<AccountId, Currency, FineArithmetic> { Flat(account, currency), PercentofAmount(account, permill, currency), }

impl<AccountId: Clone, Hash: Clone, Currency: Clone> DepositInfo<AccountId, Hash, Currency> {
    pub fn new(
        depositer: AccountId,
        reason: Hash,
        amount: Currency,
    ) -> DepositInfo<AccountId, Hash, Currency> {
        DepositInfo {
            depositer,
            reason,
            amount,
        }
    }
    pub fn depositer(&self) -> AccountId {
        self.depositer.clone()
    }
    pub fn reason(&self) -> Hash {
        self.reason.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This provides record of reservation of capital for specific purpose,
/// it makes `free` capital illiquid and effectively transfers control over this capital
/// - in v1, acceptance_committee only requires membership to authorize internal transfers which actually transfer withdrawal control
pub struct ReservationInfo<Hash, Currency, GovernanceConfig> {
    // the reason for the reservation, an ipfs reference
    reason: Hash,
    // the amount reserved
    amount: Currency,
    // the amount committed, must be less than or equal to reserved
    committed: Option<Currency>,
    // the committee for transferring liquidity rights to this capital and possibly enabling liquidity
    controller: GovernanceConfig,
}

impl<
        Hash: Clone,
        Currency: Clone
            + sp_std::ops::Add<Output = Currency>
            + sp_std::ops::Sub<Output = Currency>
            + PartialOrd,
        GovernanceConfig: Clone,
    > ReservationInfo<Hash, Currency, GovernanceConfig>
{
    pub fn new(
        reason: Hash,
        amount: Currency,
        controller: GovernanceConfig,
    ) -> ReservationInfo<Hash, Currency, GovernanceConfig> {
        ReservationInfo {
            reason,
            amount,
            committed: None,
            controller,
        }
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn committed(&self) -> Option<Currency> {
        self.committed.clone()
    }
    pub fn reason(&self) -> Hash {
        self.reason.clone()
    }
    pub fn controller(&self) -> GovernanceConfig {
        self.controller.clone()
    }
}

impl<
        Hash: Clone,
        Currency: Clone
            + sp_std::ops::Add<Output = Currency>
            + sp_std::ops::Sub<Output = Currency>
            + PartialOrd,
        GovernanceConfig: Clone,
    > CommitSpendReservation<Currency> for ReservationInfo<Hash, Currency, GovernanceConfig>
{
    fn commit_spend_reservation(
        &self,
        amount: Currency,
    ) -> Option<ReservationInfo<Hash, Currency, GovernanceConfig>> {
        if let Some(amount_committed_already) = self.committed() {
            let new_committed = amount_committed_already + amount;
            if new_committed <= self.amount() {
                Some(ReservationInfo {
                    reason: self.reason(),
                    amount: self.amount(),
                    committed: Some(new_committed),
                    controller: self.controller(),
                })
            } else {
                None
            }
        } else if amount <= self.amount() {
            Some(ReservationInfo {
                reason: self.reason(),
                amount: self.amount(),
                committed: Some(amount),
                controller: self.controller(),
            })
        } else {
            None
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone
            + sp_std::ops::Add<Output = Currency>
            + sp_std::ops::Sub<Output = Currency>
            + PartialOrd,
        GovernanceConfig: Clone,
    > MoveFundsOutUnCommittedOnly<Currency> for ReservationInfo<Hash, Currency, GovernanceConfig>
{
    fn move_funds_out_uncommitted_only(&self, amount: Currency) -> Option<Self> {
        let uncommitted_amount = if let Some(committed_count) = self.committed() {
            // ASSUMED INVARIANT, should hold here (if it doesn't ever,
            // we're mixing error branches here w/ two sources for output None)
            if self.amount() >= committed_count {
                self.amount() - committed_count
            } else {
                return None;
            }
        } else {
            self.amount()
        };
        // difference between this impl and InternalTransferInfo is this is restricted by uncommitted value
        // -- it returns None if the uncommitted_amount < the_amount_requested
        if uncommitted_amount >= amount {
            let new_amount = uncommitted_amount - amount;
            Some(ReservationInfo {
                reason: self.reason(),
                amount: new_amount,
                committed: self.committed(),
                controller: self.controller(),
            })
        } else {
            None
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone
            + sp_std::ops::Add<Output = Currency>
            + sp_std::ops::Sub<Output = Currency>
            + PartialOrd,
        GovernanceConfig: Clone,
    > MoveFundsOutCommittedOnly<Currency> for ReservationInfo<Hash, Currency, GovernanceConfig>
{
    fn move_funds_out_committed_only(&self, amount: Currency) -> Option<Self> {
        let committed_amount = self.committed()?;
        // difference between this impl and ReservationInfo is this is restricted by committed value
        // -- it returns None if the committed_amount < the_amount_requested
        if committed_amount >= amount {
            let new_amount = self.amount() - amount.clone();
            let new_committed = committed_amount - amount;
            Some(ReservationInfo {
                reason: self.reason(),
                amount: new_amount,
                committed: Some(new_committed),
                controller: self.controller(),
            })
        } else {
            None
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// Transfers withdrawal control to the new_controller
/// - them referencing this item in storage is the authentication necessary for withdrawals from the Bank
pub struct InternalTransferInfo<Hash, Currency, GovernanceConfig> {
    // the referenced Reservation from which this originated
    reference_id: u32,
    // the reason for this transfer
    reason: Hash,
    // the amount transferred
    amount: Currency,
    // governance type, should be possible to liquidate to the accounts that comprise this `GovernanceConfig` (dispatch proportional payment)
    controller: GovernanceConfig,
}

impl<Hash: Clone, Currency: Clone, GovernanceConfig: Clone>
    InternalTransferInfo<Hash, Currency, GovernanceConfig>
{
    pub fn new(
        reference_id: u32,
        reason: Hash,
        amount: Currency,
        controller: GovernanceConfig,
    ) -> InternalTransferInfo<Hash, Currency, GovernanceConfig> {
        InternalTransferInfo {
            reference_id,
            reason,
            amount,
            controller,
        }
    }
    pub fn id(&self) -> u32 {
        self.reference_id
    }
    pub fn reason(&self) -> Hash {
        self.reason.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn controller(&self) -> GovernanceConfig {
        self.controller.clone()
    }
}

impl<
        Hash: Clone,
        Currency: Clone + sp_std::ops::Sub<Output = Currency> + PartialOrd,
        GovernanceConfig: Clone,
    > MoveFundsOutCommittedOnly<Currency>
    for InternalTransferInfo<Hash, Currency, GovernanceConfig>
{
    fn move_funds_out_committed_only(
        &self,
        amount: Currency,
    ) -> Option<InternalTransferInfo<Hash, Currency, GovernanceConfig>> {
        if self.amount() >= amount {
            let new_amount = self.amount() - amount;
            Some(InternalTransferInfo {
                reference_id: self.id(),
                reason: self.reason(),
                amount: new_amount,
                controller: self.controller(),
            })
        } else {
            // not enough funds
            None
        }
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
