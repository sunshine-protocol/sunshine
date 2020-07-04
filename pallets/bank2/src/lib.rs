#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Bank2 prototype

// TODO: remove once/if this is merged into pallet-utils
#[macro_use]
extern crate derive_new;

// #[cfg(test)]
// mod tests;

use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        ReservableCurrency,
    },
};
use frame_system::{
    self as system,
    ensure_signed,
};
use sp_runtime::{
    traits::{
        AccountIdConversion,
        Zero,
    },
    DispatchError,
    DispatchResult,
};
use sp_std::prelude::*;
use util::{
    bank::OnChainTreasuryID,
    traits::{
        GenerateUniqueID,
        GroupMembership,
        IDIsAvailable,
        Increment,
        OrganizationSupervisorPermissions,
    },
};

mod bank;
use bank::{
    traits::{
        BankPermissions,
        OpenBankAccount,
        SpendFromBank,
        TransferToBank,
    },
    BankState,
};

/// The balances type for this module
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + org::Trait + donate::Trait + vote::Trait
{
    /// The overarching event types
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The limit on how many bank accounts an org can have
    type MaxTreasuryPerOrg: Get<u32>;

    /// The minimum amount necessary to open an organizational bank account
    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        Balance = BalanceOf<T>,
    {
        BankAccountOpened(AccountId, OnChainTreasuryID, Balance, OrgId, Option<AccountId>),
        SpendFromFree(AccountId, OnChainTreasuryID, AccountId, Balance),
        SpendFromReserved(AccountId, OnChainTreasuryID, AccountId, Balance),
        BankAccountClosed(AccountId, OnChainTreasuryID, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg,
        NotEnoughFreeFundsToMakeFreeSpend,
        NotEnoughReservedFundsToMakeReservedSpend,
        CannotTransferToBankThatDNE,
        CannotSpendFromBankThatDNE,
        CannotCloseBankThatDNE,
        NotPermittedToOpenBankAccountForOrg,
        NotPermittedToSpendFromReserved,
        NotPermittedToSpendFromFree,
        BankDNE,
        MustBeOrgSupervisorToCloseBankAccount,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank2 {
        /// Counter for generating unique treasury identifiers
        TreasuryIDNonce get(fn treasury_id_nonce): OnChainTreasuryID;

        /// Total number of banks registered in this module
        pub TotalBankCount get(fn total_bank_count): u32;

        /// The total number of treasury accounts per org
        pub OrgTreasuryCount get(fn org_treasury_count): map
            hasher(blake2_128_concat) T::OrgId => u32;

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique `OnChainTreasuryID`s (note the cryptographic hash function)
        pub BankStores get(fn bank_stores): map
            hasher(opaque_blake2_256) OnChainTreasuryID =>
            Option<BankState<T::AccountId, T::OrgId, BalanceOf<T>>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn open_org_bank_account(
            origin,
            org: T::OrgId,
            deposit: BalanceOf<T>,
            controller: Option<T::AccountId>,
        ) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            let auth = Self::can_open_bank_account_for_org(org, &opener);
            ensure!(auth, Error::<T>::NotPermittedToOpenBankAccountForOrg);
            let bank_id = Self::open_bank_account(opener.clone(), org, deposit, controller.clone())?;
            Self::deposit_event(RawEvent::BankAccountOpened(opener, bank_id, deposit, org, controller));
            Ok(())
        }
        #[weight = 0]
        fn spend_from_bank_free_balance(
            origin,
            bank_id: OnChainTreasuryID,
            dest: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_spend_from_free(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToSpendFromFree);
            Self::spend_from_free(bank_id, dest.clone(), amount)?;
            Self::deposit_event(RawEvent::SpendFromFree(caller, bank_id, dest, amount));
            Ok(())
        }
        #[weight = 0]
        fn spend_from_bank_reserved_balance(
            origin,
            bank_id: OnChainTreasuryID,
            dest: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_spend_from_reserved(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToSpendFromReserved);
            Self::spend_from_reserved(bank_id, dest.clone(), amount)?;
            Self::deposit_event(RawEvent::SpendFromReserved(caller, bank_id, dest, amount));
            Ok(())
        }
        #[weight = 0]
        fn close_org_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            let bank = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::CannotCloseBankThatDNE)?;
            // permissions for closing bank accounts is org supervisor status
            ensure!(
                <org::Module<T>>::is_organization_supervisor(bank.org(), &closer),
                Error::<T>::MustBeOrgSupervisorToCloseBankAccount
            );
            let bank_account_id = Self::account_id(bank_id);
            let remaining_funds = <T as donate::Trait>::Currency::total_balance(&bank_account_id);
            // distributes remaining funds equally among members
            <donate::Module<T>>::donate(
                &bank_account_id,
                bank.org(),
                remaining_funds,
                false,
            )?;
            <BankStores<T>>::remove(bank_id);
            <OrgTreasuryCount<T>>::mutate(bank.org(), |count| *count -= 1);
            <TotalBankCount>::mutate(|count| *count -= 1);
            Self::deposit_event(RawEvent::BankAccountClosed(closer, bank_id, bank.org()));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_bank(id: OnChainTreasuryID) -> bool {
        !Self::id_is_available(id)
    }
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    pub fn bank_balance(bank: OnChainTreasuryID) -> BalanceOf<T> {
        <T as Trait>::Currency::total_balance(&Self::account_id(bank))
    }
}

impl<T: Trait> IDIsAvailable<OnChainTreasuryID> for Module<T> {
    fn id_is_available(id: OnChainTreasuryID) -> bool {
        <BankStores<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<OnChainTreasuryID> for Module<T> {
    fn generate_unique_id() -> OnChainTreasuryID {
        let mut treasury_nonce_id = TreasuryIDNonce::get().increment();
        while !Self::id_is_available(treasury_nonce_id) {
            treasury_nonce_id = treasury_nonce_id.increment();
        }
        TreasuryIDNonce::put(treasury_nonce_id);
        treasury_nonce_id
    }
}

// adds a storage lookup /forall methods but this is just for local permissions anyway
impl<T: Trait> BankPermissions<OnChainTreasuryID, T::OrgId, T::AccountId>
    for Module<T>
{
    fn can_open_bank_account_for_org(
        org: T::OrgId,
        who: &T::AccountId,
    ) -> bool {
        <org::Module<T>>::is_member_of_group(org, who)
    }
    fn can_spend_from_free(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank).ok_or(Error::<T>::BankDNE)?;
        Ok(bank.is_controller(who))
    }
    fn can_reserve_spend(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank).ok_or(Error::<T>::BankDNE)?;
        Ok(bank.is_controller(who))
    }
    fn can_spend_from_reserved(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank).ok_or(Error::<T>::BankDNE)?;
        Ok(bank.is_controller(who))
    }
}

impl<T: Trait> OpenBankAccount<T::OrgId, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    type BankId = OnChainTreasuryID;
    fn open_bank_account(
        opener: T::AccountId,
        org: T::OrgId,
        deposit: BalanceOf<T>,
        controller: Option<T::AccountId>,
    ) -> Result<Self::BankId, DispatchError> {
        ensure!(
            deposit >= T::MinimumInitialDeposit::get(),
            Error::<T>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum
        );
        let new_org_bank_count = <OrgTreasuryCount<T>>::get(org) + 1;
        ensure!(
            new_org_bank_count <= T::MaxTreasuryPerOrg::get(),
            Error::<T>::CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg
        );
        // generate new treasury identifier
        let new_treasury_id = Self::generate_unique_id();
        // create new bank object
        let new_bank = BankState::new(
            org,
            deposit,
            BalanceOf::<T>::zero(),
            controller.clone(),
        );
        // perform fallible transfer
        <T as Trait>::Currency::transfer(
            &opener,
            &Self::account_id(new_treasury_id),
            deposit,
            ExistenceRequirement::KeepAlive,
        )?;
        // insert new bank object
        <BankStores<T>>::insert(new_treasury_id, new_bank);
        // iterate org treasury count
        <OrgTreasuryCount<T>>::insert(org, new_org_bank_count);
        // iterate total bank count
        <TotalBankCount>::mutate(|count| *count += 1u32);
        // return new treasury identifier
        Ok(new_treasury_id)
    }
}

impl<T: Trait> TransferToBank<T::OrgId, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    fn transfer_to_free(
        from: T::AccountId,
        bank_id: Self::BankId,
        amount: BalanceOf<T>,
    ) -> sp_runtime::DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotTransferToBankThatDNE)?
            .add_free(amount);
        <T as Trait>::Currency::transfer(
            &from,
            &Self::account_id(bank_id),
            amount,
            ExistenceRequirement::KeepAlive,
        )?;
        <BankStores<T>>::insert(bank_id, bank);
        Ok(())
    }
    fn transfer_to_reserved(
        from: T::AccountId,
        bank_id: Self::BankId,
        amount: BalanceOf<T>,
    ) -> sp_runtime::DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotTransferToBankThatDNE)?
            .add_reserved(amount);
        <T as Trait>::Currency::transfer(
            &from,
            &Self::account_id(bank_id),
            amount,
            ExistenceRequirement::KeepAlive,
        )?;
        <BankStores<T>>::insert(bank_id, bank);
        Ok(())
    }
}

impl<T: Trait> SpendFromBank<T::OrgId, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    fn spend_from_free(
        bank_id: Self::BankId,
        dest: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSpendFromBankThatDNE)?
            .subtract_free(amount)
            .ok_or(Error::<T>::NotEnoughFreeFundsToMakeFreeSpend)?;
        <T as Trait>::Currency::transfer(
            &Self::account_id(bank_id),
            &dest,
            amount,
            ExistenceRequirement::KeepAlive,
        )?;
        <BankStores<T>>::insert(bank_id, bank);
        Ok(())
    }
    fn spend_from_reserved(
        bank_id: Self::BankId,
        dest: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSpendFromBankThatDNE)?
            .subtract_reserved(amount)
            .ok_or(Error::<T>::NotEnoughReservedFundsToMakeReservedSpend)?;
        <T as Trait>::Currency::transfer(
            &Self::account_id(bank_id),
            &dest,
            amount,
            ExistenceRequirement::KeepAlive,
        )?;
        <BankStores<T>>::insert(bank_id, bank);
        Ok(())
    }
}
