#![recursion_limit = "256"]
//! # Drip Module
//! This module expresses logic for scheduling a stream of transfers from a sender
//! to a recipient. It treats the transfer as continuous rather than discrete with
//! a flow rate of X Balance every N Blocks.
//!
//! - [`drip::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet enables scheduling recurring payments at a specified rate of
//! dripping from the sender to recipient.
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
    decl_storage,
    ensure,
    storage::IterableStorageMap,
    traits::{
        Currency,
        ExistenceRequirement,
    },
    Parameter,
};
use frame_system::{
    ensure_signed,
    Trait as System,
};
use parity_scale_codec::Codec;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        CheckedDiv,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchResult,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    drip::{
        Drip,
        DripRate,
    },
    traits::{
        GenerateUniqueID,
        IDIsAvailable,
    },
};

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as System>::AccountId>>::Balance;
type DripOf<T> = Drip<
    <T as System>::AccountId,
    DripRate<<T as System>::BlockNumber, BalanceOf<T>>,
>;
pub trait Trait: System {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// The identifier for drips
    type DripId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// Currency type
    type Currency: Currency<Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as Trait>::DripId,
        <T as System>::AccountId,
        <T as System>::BlockNumber,
        Balance = BalanceOf<T>,
    {
        /// Drip identifier, First payment block, Source, Destination, Amount Per Period, Period Length
        DripStarted(DripId, BlockNumber, AccountId, AccountId, Balance, BlockNumber),
        /// Drip from Source to Destination of Amount
        Dripped(AccountId, AccountId, Balance),
        /// Drip identifier at this BlockNumber with this drip info
        DripCancelled(DripId, BlockNumber, AccountId, AccountId, Balance, BlockNumber),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        DoNotDripToSelf,
        RatePeriodLengthMustBeGreaterThanZero,
        RateAmountMustBeGreaterThanZero,
        DripDNE,
        NotAuthorizedToCancelDrip,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Drip {
        /// The nonce for unique drip id generation
        DripIdCounter get(fn drip_id_counter): T::DripId;

        /// The number of open drips
        pub OpenDripCounter get(fn open_drip_counter): u32;

        /// The state of drips
        pub Drips get(fn drips): map
            hasher(blake2_128_concat) T::DripId => Option<DripOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn start_drip(
            origin,
            destination: T::AccountId,
            rate: DripRate<T::BlockNumber, BalanceOf<T>>,
        ) -> DispatchResult {
            let source = ensure_signed(origin)?;
            let first_payment_block = Self::first_next_block_mod_period_is_zero(rate.period_length())
                .ok_or(Error::<T>::RatePeriodLengthMustBeGreaterThanZero)?;
            ensure!(source != destination, Error::<T>::DoNotDripToSelf);
            ensure!(rate.amount() > 0u32.into(), Error::<T>::RateAmountMustBeGreaterThanZero);
            let drip = Drip::new(source.clone(), destination.clone(), rate);
            let id = Self::generate_unique_id();
            <Drips<T>>::insert(id, drip);
            OpenDripCounter::mutate(|n| *n += 1u32);
            Self::deposit_event(
                RawEvent::DripStarted(
                    id,
                    first_payment_block,
                    source,
                    destination,
                    rate.amount(),
                    rate.period_length()
                )
            );
            Ok(())
        }

        #[weight = 0]
        fn cancel_drip(
            origin,
            id: T::DripId
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let drip = <Drips<T>>::get(id).ok_or(Error::<T>::DripDNE)?;
            ensure!(drip.source() == caller, Error::<T>::NotAuthorizedToCancelDrip);
            <Drips<T>>::remove(id);
            OpenDripCounter::mutate(|n| *n -= 1u32);
            let now = <frame_system::Module<T>>::block_number();
            Self::deposit_event(
                RawEvent::DripCancelled(
                    id,
                    now,
                    drip.source(),
                    drip.destination(),
                    drip.rate().amount(),
                    drip.rate().period_length()
                )
            );
            Ok(())
        }

        fn on_finalize(_n: T::BlockNumber) {
           let current_block = <frame_system::Module<T>>::block_number();
           // TODO: sweep periodically instead of scanning after every block (which is what it does now)
            <Drips<T>>::iter()
                .filter(|(_, drip)| current_block % drip.rate().period_length() == 0u32.into())
                .for_each(|(_, drip)| Self::pay(drip));
        }
    }
}

impl<T: Trait> Module<T> {
    fn first_next_block_mod_period_is_zero(
        period_length: T::BlockNumber,
    ) -> Option<T::BlockNumber> {
        let now = <frame_system::Module<T>>::block_number();
        if let Some(div) = now.checked_div(&period_length) {
            let a = div * period_length;
            if a > now {
                Some(a)
            } else {
                let b = period_length - a;
                Some(now + b)
            }
        } else {
            None
        }
    }
    fn pay(drip: Drip<T::AccountId, DripRate<T::BlockNumber, BalanceOf<T>>>) {
        let (src, dest, amt) =
            (&drip.source(), &drip.destination(), drip.rate().amount());
        if T::Currency::transfer(
            src,
            dest,
            amt,
            ExistenceRequirement::KeepAlive,
        )
        .is_ok()
        {
            Self::deposit_event(RawEvent::Dripped(
                drip.source(),
                drip.destination(),
                drip.rate().amount(),
            ));
        } // TODO: should notify source and dest accounts somehow for error branch
    }
}

impl<T: Trait> IDIsAvailable<T::DripId> for Module<T> {
    fn id_is_available(id: T::DripId) -> bool {
        <Drips<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<T::DripId> for Module<T> {
    fn generate_unique_id() -> T::DripId {
        let mut id_counter = <DripIdCounter<T>>::get() + 1u32.into();
        while <Drips<T>>::get(id_counter).is_some() {
            id_counter += 1u32.into();
        }
        <DripIdCounter<T>>::put(id_counter);
        id_counter
    }
}
