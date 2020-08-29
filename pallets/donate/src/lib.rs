#![recursion_limit = "256"]
//! # Donate Module
//! This module expresses logic for transferring funds to a set of `AccountId`s
//! in proportion to their `Shares` ownership in the Org. It allows for donating
//! 1. in proportion to `Shares` ownership in the org
//! 2. an equal amount for each account in the org
//!
//! There is no perfect way to deal with remainders so the caller must
//! include an `AccountId` to which the remainder will go.
//!
//! - [`donate::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet enables transfers to Orgs as a weighted set of accounts. It demonstrates
//! a batch transfer with each recipient receiving an amount in proportion to
//! their relative `Shares` ownership.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html
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
        ReservableCurrency,
    },
};
use frame_system::{
    self as system,
    ensure_signed,
};
use sp_runtime::{
    traits::{
        CheckedSub,
        Zero,
    },
    DispatchError,
    DispatchResult,
    Permill,
};
use util::{
    organization::OrgRep,
    traits::GetGroup,
};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as system::Trait>::AccountId,
>>::Balance;

pub trait Trait: system::Trait + org::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The currency type
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        Balance = BalanceOf<T>,
    {
        PropDonationExecuted(AccountId, Balance, OrgId, Balance, AccountId),
        EqualDonationExecuted(AccountId, Balance, OrgId, Balance, AccountId),
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
        fn make_prop_donation(
            origin,
            org: T::OrgId,
            remainder_recipient: T::AccountId,
            amt: BalanceOf<T>
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let (
                amt_transferred_to_org,
                remainder_transferred_to_acc
            ) = Self::donate(&sender, OrgRep::Weighted(org), &remainder_recipient, amt)?;
            Self::deposit_event(
                RawEvent::PropDonationExecuted(
                    sender,
                    amt_transferred_to_org,
                    org,
                    remainder_transferred_to_acc,
                    remainder_recipient,
                )
            );
            Ok(())
        }
        #[weight = 0]
        fn make_equal_donation(
            origin,
            org: T::OrgId,
            remainder_recipient: T::AccountId,
            amt: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            let (
                amt_transferred_to_org,
                remainder_transferred_to_acc
            ) = Self::donate(&sender, OrgRep::Equal(org), &remainder_recipient, amt)?;
            Self::deposit_event(
                RawEvent::EqualDonationExecuted(
                    sender,
                    amt_transferred_to_org,
                    org,
                    remainder_transferred_to_acc,
                    remainder_recipient,
                )
            );
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Returns the remainder NOT transferred because the amount was not perfectly divisible
    pub fn donate(
        sender: &T::AccountId,
        recipient: OrgRep<T::OrgId>,
        remainder_recipient: &T::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
        let free = T::Currency::free_balance(sender);
        // TODO: add buffer for expected transaction fee? depends on shape of tx
        let _ = free
            .checked_sub(&amt)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToMakeTransfer)?;
        // match on recipient type to distribute the donation either in proportion
        // to org ownership or equally among all members
        let remainder = match recipient {
            OrgRep::Weighted(org_id) => {
                // Get the membership set of the Org
                let group = <org::Module<T>>::get_group(org_id)
                    .ok_or(Error::<T>::CannotDonateToOrgThatDNE)?;
                // iterate through and pay the transfer
                let mut transferred_amt = BalanceOf::<T>::zero();
                group
                    .0
                    .into_iter()
                    .map(|acc: T::AccountId| -> DispatchResult {
                        let amt_due = Self::calculate_proportional_amount(
                            amt,
                            acc.clone(),
                            org_id,
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
                amt - transferred_amt
            }
            OrgRep::Equal(org_id) => {
                // Get the membership set of the Org
                let group = <org::Module<T>>::get_group(org_id)
                    .ok_or(Error::<T>::CannotDonateToOrgThatDNE)?;
                // amount for each member if equal payment per member
                let equal_payment =
                    Self::calculate_uniform_amount(amt, group.0.len())?;
                // iterate through and pay the transfer
                let mut transferred_amt = BalanceOf::<T>::zero();
                group
                    .0
                    .into_iter()
                    .map(|acc: T::AccountId| -> DispatchResult {
                        T::Currency::transfer(
                            sender,
                            &acc,
                            equal_payment,
                            ExistenceRequirement::KeepAlive,
                        )?;
                        transferred_amt += equal_payment;
                        Ok(())
                    })
                    .collect::<DispatchResult>()?;
                amt - transferred_amt
            }
        };
        // transfer remainder to remainder recipient
        T::Currency::transfer(
            sender,
            remainder_recipient,
            remainder,
            ExistenceRequirement::KeepAlive,
        )?;
        let amt_transferred_to_org = amt - remainder;
        Ok((amt_transferred_to_org, remainder))
    }
    fn calculate_proportional_amount(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: T::OrgId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let issuance = <org::Module<T>>::orgs(group)
            .ok_or(Error::<T>::CannotDonateToOrgThatDNE)?
            .total_shares();
        let acc_ownership = <org::Module<T>>::members(group, &account)
            .ok_or(Error::<T>::AccountHasNoOwnershipInOrg)?;
        let ownership = Permill::from_rational_approximation(
            acc_ownership.total(),
            issuance,
        );
        Ok(ownership.mul_floor(amount))
    }
    fn calculate_uniform_amount(
        amount: BalanceOf<T>,
        group_count: usize,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let group_size: u32 = group_count as u32;
        let equal_ownership =
            Permill::from_rational_approximation(1u32, group_size);
        Ok(equal_ownership.mul_floor(amount))
    }
}
