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
    AccountId,
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    Currency,
> {
    // Registered organization identifier
    org: OrgId,
    // Free for spending
    free: Currency,
    // Reserved for future spending
    reserved: Currency,
    // Layered sudo, selection should eventually be revocable by the group
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
    pub fn add_free(&self, amt: Currency) -> Self {
        let new_amt = self.free() + amt;
        BankState {
            free: new_amt,
            ..self.clone()
        }
    }
    pub fn subtract_free(&self, amt: Currency) -> Option<Self> {
        if self.free() >= amt {
            let new_amt = self.free() - amt;
            Some(BankState {
                free: new_amt,
                ..self.clone()
            })
        } else {
            None
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

pub mod traits {
    pub type Result<T> = sp_std::result::Result<T, sp_runtime::DispatchError>;

    // local permissions for this module
    pub trait BankPermissions<BankId, OrgId, AccountId> {
        fn can_open_bank_account_for_org(org: OrgId, who: &AccountId) -> bool;
        fn can_spend_from_free(bank: BankId, who: &AccountId) -> Result<bool>;
        fn can_reserve_spend(bank: BankId, who: &AccountId) -> Result<bool>;
        fn can_spend_from_reserved(
            bank: BankId,
            who: &AccountId,
        ) -> Result<bool>;
    }

    pub trait OpenBankAccount<OrgId, Currency, AccountId> {
        type BankId;
        fn open_bank_account(
            opener: AccountId,
            org: OrgId,
            deposit: Currency,
            controller: Option<AccountId>,
        ) -> Result<Self::BankId>;
    }

    pub trait TransferToBank<OrgId, Currency, AccountId>:
        OpenBankAccount<OrgId, Currency, AccountId>
    {
        fn transfer_to_free(
            from: AccountId,
            bank_id: Self::BankId,
            amount: Currency,
        ) -> sp_runtime::DispatchResult;
        fn transfer_to_reserved(
            from: AccountId,
            bank_id: Self::BankId,
            amount: Currency,
        ) -> sp_runtime::DispatchResult;
    }

    pub trait SpendFromBank<OrgId, Currency, AccountId>:
        OpenBankAccount<OrgId, Currency, AccountId>
    {
        fn spend_from_free(
            bank_id: Self::BankId,
            dest: AccountId,
            amount: Currency,
        ) -> sp_runtime::DispatchResult;
        fn spend_from_reserved(
            bank_id: Self::BankId,
            dest: AccountId,
            amount: Currency,
        ) -> sp_runtime::DispatchResult;
    }
}
