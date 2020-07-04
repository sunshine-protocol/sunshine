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
    },
};

mod bank;
use bank::{
    traits::{
        OpenBankAccount,
        SpendFromBank,
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
        // ProposeSpend(),
        SpendFromFree(AccountId, OnChainTreasuryID, AccountId, Balance),
        SpendFromReserved(AccountId, OnChainTreasuryID, AccountId, Balance),
        BankAccountClosed(AccountId, OnChainTreasuryID, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotOpenBankAccountForOrgIfNotOrgMember,
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg,
        CallerNotAuthorizedToMakeSpendFromBank,
        NotEnoughFreeFundsToMakeFreeSpend,
        NotEnoughReservedFundsToMakeReservedSpend,
        CannotSpendFromBankThatDNE,
        CannotCloseBankThatDNE,
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
            Self::spend_from_free(caller.clone(), bank_id, dest.clone(), amount)?;
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
            Self::spend_from_reserved(caller.clone(), bank_id, dest.clone(), amount)?;
            Self::deposit_event(RawEvent::SpendFromReserved(caller, bank_id, dest, amount));
            Ok(())
        }
        #[weight = 0]
        fn close_org_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            // TODO: auth
            let bank = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::CannotCloseBankThatDNE)?;
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
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    pub fn is_bank(id: OnChainTreasuryID) -> bool {
        !Self::id_is_available(id)
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
        let authentication = <org::Module<T>>::is_member_of_group(org, &opener);
        ensure!(
            authentication,
            Error::<T>::CannotOpenBankAccountForOrgIfNotOrgMember
        );
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

impl<T: Trait> SpendFromBank<T::OrgId, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    fn spend_from_free(
        caller: T::AccountId,
        bank_id: Self::BankId,
        dest: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSpendFromBankThatDNE)?
            .subtract_free(amount)
            .ok_or(Error::<T>::NotEnoughFreeFundsToMakeFreeSpend)?;
        ensure!(
            bank.is_controller(&caller),
            Error::<T>::CallerNotAuthorizedToMakeSpendFromBank
        );
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
        caller: T::AccountId,
        bank_id: Self::BankId,
        dest: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSpendFromBankThatDNE)?
            .subtract_reserved(amount)
            .ok_or(Error::<T>::NotEnoughReservedFundsToMakeReservedSpend)?;
        ensure!(
            bank.is_controller(&caller),
            Error::<T>::CallerNotAuthorizedToMakeSpendFromBank
        );
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
