#![cfg_attr(not(feature = "std"), no_std)]

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
    },
    DispatchError,
    DispatchResult,
    ModuleId,
};
use util::traits::{
    CalculateOwnership,
    GetGroup,
};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as system::Trait>::AccountId,
>>::Balance;

pub trait Trait: system::Trait + org::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The currency type
    type Currency: Currency<Self::AccountId>;
    //// Taxes for using this module
    type TransactionFee: Get<BalanceOf<Self>>;
    /// Where the taxes go (should be a treasury identifier)
    type Treasury: Get<ModuleId>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        Balance = BalanceOf<T>,
    {
        DonationExecuted(AccountId, OrgId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NotEnoughFundsInFreeToMakeTransfer,
        CannotDonateToOrgThatDNE,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        #[weight = 0]
        fn make_donation_in_proportion_to_ownership(
            origin,
            org: T::OrgId,
            amt: BalanceOf<T>
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::donate(&sender, org, amt)?;
            Self::deposit_event(RawEvent::DonationExecuted(sender, org, amt));
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
    pub fn donate(
        sender: &T::AccountId,
        recipient: T::OrgId,
        amt: BalanceOf<T>,
    ) -> DispatchResult {
        let free = T::Currency::free_balance(sender);
        let total_transfer = amt + T::TransactionFee::get();
        let _ = free
            .checked_sub(&total_transfer)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToMakeTransfer)?;
        // Get the membership set of the Org
        let group = <org::Module<T>>::get_group(recipient)
            .ok_or(Error::<T>::CannotDonateToOrgThatDNE)?;
        // iterate through and pay the transfer out
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
                Ok(())
            })
            .collect::<DispatchResult>()?;
        // pay the transaction fee last
        T::Currency::transfer(
            &sender,
            &Self::account_id(),
            T::TransactionFee::get(),
            ExistenceRequirement::KeepAlive,
        )
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: T::OrgId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let proportion_due =
            <org::Module<T>>::calculate_proportion_ownership_for_account(
                account, group,
            )?;
        Ok(proportion_due * amount)
    }
}
