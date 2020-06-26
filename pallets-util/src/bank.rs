use crate::traits::{
    Increment,
    SpendWithdrawOps,
};
use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_core::TypeId;
use sp_runtime::traits::{
    AtLeast32Bit,
    Zero,
};
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
pub struct FullBankId<T> {
    pub id: OnChainTreasuryID,
    pub sub_id: T,
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
/// The identifier for the submaps in this module
pub enum BankMapId {
    Transfer,
    ReserveSpend,
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum Sender<AccountId, OrgId> {
    Account(AccountId),
    Org(OrgId),
}
#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
// same object as `Sender`, yes, but not if we need to start adding trait bounds or something and it's just easier to read with different names
pub enum Recipient<AccountId, BankId> {
    Account(AccountId),
    Bank(BankId),
}

#[derive(
    new, PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug,
)]
/// This is the state for an OnChainBankId, associated with each bank registered in the runtime
pub struct BankState<
    AccountId,
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    Currency,
> {
    // Registered organization identifier
    org: OrgId,
    // Sum of all reserved spending that has not been executed
    // -> free = T::Currency::total_balance(&Self::account_id(bank_id)) - reserved
    reserved: Currency,
    // Sudo, if not set, defaults to Org's sudo and if that's not set, just false
    controller: Option<AccountId>,
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Clone
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
    > BankState<AccountId, OrgId, Currency>
{
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn reserved(&self) -> Currency {
        self.reserved.clone()
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
    pub fn add_reserved(&self, amt: Currency) -> Self {
        let new_amt = self.reserved() + amt;
        BankState {
            reserved: new_amt,
            ..self.clone()
        }
    }
    pub fn subtract_reserved(&self, amt: Currency) -> Option<Self> {
        if self.reserved() >= amt {
            let new_amt = self.reserved() - amt;
            Some(BankState {
                reserved: new_amt,
                ..self.clone()
            })
        } else {
            None
        }
    }
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum TransferState {
    /// Locked transfers s.t. payment can be revoked by the sender
    Locked,
    /// Free for org related spending or reserving future spends
    FreeForSpending,
    /// Remaining funds may be withdrawn by members in proportion to ownership
    WithdrawableByMembers,
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct TransferInformation<Sender, Currency, State> {
    // the sender
    sender: Sender,
    // the amount received by the recipient initially
    initial_amount: Currency,
    // spent or reserved
    amount_spent: Currency,
    // amount withdrawn by members
    amount_claimed: Currency,
    // the state which determines what interactions are permitted
    state: State,
}

impl<Sender: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    TransferInformation<Sender, Currency, TransferState>
{
    pub fn new(sender: Sender, amt: Currency) -> Self {
        TransferInformation {
            sender,
            initial_amount: amt,
            amount_spent: Currency::zero(),
            amount_claimed: Currency::zero(),
            state: TransferState::FreeForSpending,
        }
    }
    pub fn sender(&self) -> Sender {
        self.sender.clone()
    }
    pub fn initial_amount(&self) -> Currency {
        self.initial_amount.clone()
    }
    pub fn amount_spent(&self) -> Currency {
        self.amount_spent.clone()
    }
    pub fn amount_claimed(&self) -> Currency {
        self.amount_claimed.clone()
    }
    // NEVER underflows by construction
    // (invariant enforced in object interaction; see trait impls further below)
    pub fn amount_left(&self) -> Currency {
        self.initial_amount() - self.amount_spent() - self.amount_claimed()
    }
    pub fn state(&self) -> TransferState {
        self.state
    }
}

impl<Sender: Clone + PartialEq, Currency: Zero + AtLeast32Bit + Clone>
    SpendWithdrawOps<Currency>
    for TransferInformation<Sender, Currency, TransferState>
{
    fn spend(
        &self,
        amt: Currency,
    ) -> Option<TransferInformation<Sender, Currency, TransferState>> {
        match self.state() {
            TransferState::FreeForSpending => {
                if self.amount_left() <= amt {
                    let new_spent_amt = self.amount_spent() + amt;
                    Some(TransferInformation {
                        amount_spent: new_spent_amt,
                        ..self.clone()
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
    fn withdraw(
        &self,
        amt: Currency,
    ) -> Option<TransferInformation<Sender, Currency, TransferState>> {
        match self.state() {
            TransferState::WithdrawableByMembers => {
                if self.amount_left() <= amt {
                    let new_claimed_amt = self.amount_claimed() + amt;
                    Some(TransferInformation {
                        amount_claimed: new_claimed_amt,
                        ..self.clone()
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(
    new, Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct SpendReservation<Recipient, Currency> {
    // the intended recipient for this spend reservation
    recipient: Recipient,
    // the amount received by the recipient initially
    amount: Currency,
    // spent
    amount_spent: Currency,
}

impl<
        Recipient: Clone + PartialEq,
        Currency: Clone
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
    > SpendReservation<Recipient, Currency>
{
    pub fn recipient(&self) -> Recipient {
        self.recipient.clone()
    }
    pub fn initial_amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn amount_spent(&self) -> Currency {
        self.amount_spent.clone()
    }
    pub fn amount_left(&self) -> Currency {
        self.initial_amount() - self.amount_spent()
    }
    pub fn is_recipient(&self, who: &Recipient) -> bool {
        &self.recipient == who
    }
    pub fn spend(&self, amt: Currency) -> Option<Self> {
        if self.amount_left() >= amt {
            let new_amt_spent = self.amount_spent() + amt;
            Some(SpendReservation {
                amount_spent: new_amt_spent,
                ..self.clone()
            })
        } else {
            None
        }
    }
}
