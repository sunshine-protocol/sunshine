use crate::{
    share::SimpleShareGenesis,
    traits::{
        AccessGenesis, CommitSpendReservation, DepositSpendOps, FreeToReserved, GetBalance,
        Increment, MoveFundsOutCommittedOnly, MoveFundsOutUnCommittedOnly,
    },
};
use codec::{Codec, Decode, Encode};
use sp_core::TypeId;
use sp_runtime::{
    traits::{AtLeast32Bit, Member, Zero},
    Permill,
};
use sp_std::prelude::*;

/// An on-chain treasury identifier, exactly like `ModuleId`
#[derive(Clone, Copy, Eq, PartialEq, Default, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct OnChainTreasuryID(pub [u8; 8]);

impl crate::traits::Increment for OnChainTreasuryID {
    fn increment(self) -> OnChainTreasuryID {
        let old_inner = u64::from_be_bytes(self.0);
        let new_inner = old_inner.saturating_add(1u64);
        OnChainTreasuryID(new_inner.to_be_bytes())
    }
}

impl TypeId for OnChainTreasuryID {
    const TYPE_ID: [u8; 4] = *b"bank";
}

#[derive(Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub enum Sender<AccountId, OrgId> {
    Account(AccountId),
    Org(OrgId),
}
// Recipient { OrgId }; //could augment this struct with group governance rules but not immediately

#[derive(PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// This is the state for an OnChainBankId, associated with each bank registered in the runtime
pub struct BankState<AccountId, OrgId: Codec + PartialEq + Zero + From<u32> + Copy, Currency> {
    // Registered organization identifier
    org: OrgId,
    // Free capital, available for spends
    free: Currency,
    // Set aside for future spends, already allocated but can only be _liquidated_ after free == 0?
    reserved: Currency,
    // Sudo, if not set, defaults to Org's sudo and if that's not set, just false
    controller: Option<AccountId>,
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Zero + AtLeast32Bit + Clone,
    > BankState<AccountId, OrgId, Currency>
{
    pub fn new_from_deposit(org: OrgId, amount: Currency, controller: Option<AccountId>) -> Self {
        BankState {
            org,
            free: amount,
            reserved: Currency::zero(),
            controller,
        }
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn free(&self) -> Currency {
        self.free.clone()
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
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Zero + AtLeast32Bit + Clone + sp_std::ops::Add + sp_std::ops::Sub,
    > FreeToReserved<Currency> for BankState<AccountId, OrgId, Currency>
{
    fn move_from_free_to_reserved(&self, amount: Currency) -> Option<Self> {
        if self.free() >= amount {
            // safe because of above conditional
            let new_free = self.free() - amount.clone();
            let new_reserved = self.reserved() + amount;
            Some(BankState {
                org: self.org(),
                free: new_free,
                reserved: new_reserved,
                controller: self.controller(),
            })
        } else {
            // failed, not enough in free to make reservation of amount
            None
        }
    }
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Zero + AtLeast32Bit + Clone + sp_std::ops::Add,
    > GetBalance<Currency> for BankState<AccountId, OrgId, Currency>
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

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Zero + AtLeast32Bit + Clone,
    > DepositSpendOps<Currency> for BankState<AccountId, OrgId, Currency>
{
    // infallible
    fn deposit_into_free(&self, amount: Currency) -> Self {
        let new_free = self.free() + amount;
        BankState {
            org: self.org(),
            free: new_free,
            reserved: self.reserved(),
            controller: self.controller(),
        }
    }
    fn deposit_into_reserved(&self, amount: Currency) -> Self {
        let new_reserved = self.reserved() + amount;
        BankState {
            org: self.org(),
            free: self.free(),
            reserved: new_reserved,
            controller: self.controller(),
        }
    }
    // fallible, not enough capital in relative account
    fn spend_from_free(&self, amount: Currency) -> Option<Self> {
        if self.free() >= amount {
            let new_free = self.free() - amount;
            Some(BankState {
                org: self.org(),
                free: new_free,
                reserved: self.reserved(),
                controller: self.controller(),
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
                org: self.org(),
                free: self.free(),
                reserved: new_reserved,
                controller: self.controller(),
            })
        } else {
            // not enough capital in account, spend failed
            None
        }
    }
}

#[derive(new, Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct TransferInformation<AccountId, OrgId, Currency> {
    sender: Sender<AccountId, OrgId>,
    amount_transferred: Currency,
    // amt that references this transfer for withdrawal
    amount_claimed: Currency,
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        Currency: Zero + AtLeast32Bit + Clone,
    > TransferInformation<AccountId, OrgId, Currency>
{
    pub fn sender(&self) -> Sender<AccountId, OrgId> {
        self.sender.clone()
    }
    pub fn amount_transferred(&self) -> Currency {
        self.amount_transferred.clone()
    }
    pub fn amount_claimed(&self) -> Currency {
        self.amount_claimed.clone()
    }
    pub fn claim_amount(&self, amount: Currency) -> Option<Self> {
        let condition: bool = (self.amount_transferred() - self.amount_claimed()) >= amount;
        if condition {
            let new_amount_claimed = self.amount_claimed() + amount;
            Some(TransferInformation {
                amount_claimed: new_amount_claimed,
                ..self.clone()
            })
        } else {
            None
        }
    }
}
