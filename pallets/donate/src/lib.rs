#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
    },
};
use frame_system::{
    self as system,
    ensure_signed,
};
use sp_runtime::{
    traits::{
        AccountIdConversion,
        CheckedSub,
        Zero,
    },
    DispatchError,
    DispatchResult,
    ModuleId,
    Permill,
};
use util::traits::GetGroup;

type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as system::Trait>::AccountId,
>>::Balance;

pub trait Trait: system::Trait + org::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The currency type
    type Currency: Currency<Self::AccountId>;
    //// Taxes for some transfers
    type TransactionFee: Get<BalanceOf<Self>>;
    /// Where the conditional taxes go
    type Treasury: Get<ModuleId>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        Balance = BalanceOf<T>,
    {
        DonationExecuted(AccountId, OrgId, Balance, bool),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        AccountHasNoOwnershipInOrg,
        NotEnoughFundsInFreeToMakeTransfer,
        CannotDonateToOrgThatDNE,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 0]
        fn make_prop_donation_with_fee(
            origin,
            org: T::OrgId,
            amt: BalanceOf<T>
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let remainder = Self::donate(&sender, org, amt, true)?;
            let transferred_amt = amt - remainder;
            Self::deposit_event(RawEvent::DonationExecuted(sender, org, transferred_amt, true));
            Ok(())
        }
        #[weight = 0]
        fn make_prop_donation_without_fee(
            origin,
            org: T::OrgId,
            amt: BalanceOf<T>
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let remainder = Self::donate(&sender, org, amt, false)?;
            let transferred_amt = amt - remainder;
            Self::deposit_event(RawEvent::DonationExecuted(sender, org, transferred_amt, false));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// The account ID of this module's treasury
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::Treasury::get().into_account()
    }
    /// Returns the remainder NOT transferred because the amount was not perfectly divisible
    pub fn donate(
        sender: &T::AccountId,
        recipient: T::OrgId,
        amt: BalanceOf<T>,
        transaction_fee: bool,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let free = T::Currency::free_balance(sender);
        let total_transfer = if transaction_fee {
            amt + T::TransactionFee::get()
        } else {
            amt
        };
        let _ = free
            .checked_sub(&total_transfer)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToMakeTransfer)?;
        // Get the membership set of the Org
        let group = <org::Module<T>>::get_group(recipient)
            .ok_or(Error::<T>::CannotDonateToOrgThatDNE)?;
        // iterate through and pay the transfer out
        let mut transferred_amt = BalanceOf::<T>::zero();
        group
            .0
            .into_iter()
            .map(|acc: T::AccountId| -> DispatchResult {
                let amt_due = Self::calculate_proportional_amount_for_account(
                    amt,
                    acc.clone(),
                    recipient,
                )?;
                T::Currency::transfer(
                    sender,
                    &acc,
                    amt_due,
                    ExistenceRequirement::KeepAlive,
                )?;
                transferred_amt += amt_due;
                Ok(())
            })
            .collect::<DispatchResult>()?;
        let remainder = amt - transferred_amt;
        if transaction_fee {
            // pay the transaction fee last
            T::Currency::transfer(
                &sender,
                &Self::account_id(),
                T::TransactionFee::get(),
                ExistenceRequirement::KeepAlive,
            )?;
        }
        Ok(remainder)
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: T::OrgId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let issuance = <org::Module<T>>::total_issuance(group);
        let acc_ownership = <org::Module<T>>::members(group, &account)
            .ok_or(Error::<T>::AccountHasNoOwnershipInOrg)?;
        let ownership = Permill::from_rational_approximation(
            acc_ownership.total(),
            issuance,
        );
        Ok(ownership.mul_floor(amount))
    }
}
