#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Kickback pallet for event management with incentives

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    storage::IterableStorageDoubleMap,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
    },
    Parameter,
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
        AccountIdConversion,
        AtLeast32Bit,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchError,
    DispatchResult,
    ModuleId,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::kickback::KickbackEvent;

// type aliases
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type KickbackEventFor<T> = KickbackEvent<
    <T as Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
>;

pub trait Trait: frame_system::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid type
    type IpfsReference: Parameter + Member + Default;

    /// The currency type
    type Currency: Currency<Self::AccountId>;

    /// The event identifier
    type KickbackEventId: Parameter
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

    /// The pool for all event collateral
    type EventPool: Get<ModuleId>;

    /// Minimum reservation requirement for posted events
    type MinReservationReq: Get<BalanceOf<Self>>;

    /// Maximum attendance limit for posted events
    type MaxAttendance: Get<u32>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait>::IpfsReference,
        <T as Trait>::KickbackEventId,
        Balance = BalanceOf<T>,
    {
        /// Poster, Minimum Reservation Requirement, Identifier, Event Metadata (i.e. location)
        EventPosted(AccountId, Balance, KickbackEventId, IpfsReference),
        /// Event, Reserver
        EventSeatReserved(KickbackEventId, AccountId),
        /// Event ID Closed, Reservation Requirement, Amt Returned Per Present, Remainder Sent to Publisher
        EventClosed(KickbackEventId, Balance, Balance, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Event Does Not Exist
        KickbackEventDNE,
        EventReservationReqBelowModuleMin,
        EventAttendanceLimitAboveModuleMax,
        AlreadyMadeReservation,
        AttendanceLimitReached,
        AttendanceMustBeGreaterThanZero,
        NotAuthorizedToPublishAttendance,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Kickback {
        /// Uid generation helper for KickbackEventId
        KickbackEventNonce get(fn kickback_event_nonce): T::KickbackEventId;

        /// Posted events
        pub KickbackEvents get(fn kickback_events): map
            hasher(blake2_128_concat) T::KickbackEventId => Option<KickbackEventFor<T>>;
        /// Seat reservation history
        pub KickbackReservations get(fn kickback_reservations): double_map
            hasher(blake2_128_concat) T::KickbackEventId,
            hasher(blake2_128_concat) T::AccountId => Option<()>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn post_kickback_event(
            origin,
            info: T::IpfsReference,
            reservation_req: BalanceOf<T>,
            attendance_limit: u32,
        ) -> DispatchResult {
            let supervisor = ensure_signed(origin)?;
            ensure!(reservation_req >= T::MinReservationReq::get(), Error::<T>::EventReservationReqBelowModuleMin);
            ensure!(attendance_limit <= T::MaxAttendance::get(), Error::<T>::EventAttendanceLimitAboveModuleMax);
            let kickback_event = KickbackEventFor::<T>::new(info.clone(), supervisor.clone(), reservation_req, attendance_limit);
            let id = Self::kickback_event_generate_uid();
            <KickbackEvents<T>>::insert(id, kickback_event);
            Self::deposit_event(RawEvent::EventPosted(supervisor, reservation_req, id, info));
            Ok(())
        }
        #[weight = 0]
        fn reserve_seat(
            origin,
            event_id: T::KickbackEventId,
        ) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            let kickback_event = <KickbackEvents<T>>::get(event_id).ok_or(Error::<T>::KickbackEventDNE)?;
            ensure!(<KickbackReservations<T>>::get(event_id, &reserver).is_none(), Error::<T>::AlreadyMadeReservation);
            let new_event = kickback_event.increment_attendance().ok_or(Error::<T>::AttendanceLimitReached)?;
            T::Currency::transfer(
                &reserver,
                &Self::event_account_id(event_id),
                kickback_event.reservation_req(),
                ExistenceRequirement::KeepAlive,
            )?;
            <KickbackReservations<T>>::insert(event_id, &reserver, ());
            <KickbackEvents<T>>::insert(event_id, new_event);
            Self::deposit_event(RawEvent::EventSeatReserved(event_id, reserver));
            Ok(())
        }
        #[weight = 0]
        pub fn publish_attendance_and_execute_redistribution(
            origin,
            id: T::KickbackEventId,
            present: Vec<T::AccountId>,
        ) -> DispatchResult {
            ensure!(!present.is_empty(), Error::<T>::AttendanceMustBeGreaterThanZero);
            let publisher = ensure_signed(origin)?;
            let k = <KickbackEvents<T>>::get(id).ok_or(Error::<T>::KickbackEventDNE)?;
            ensure!(k.supervisor() == publisher, Error::<T>::NotAuthorizedToPublishAttendance);
            let present_members = <KickbackReservations<T>>::iter()
                .filter(|(i, ac, _)| i == &id && present.binary_search(&ac).is_ok())
                .map(|(_, a, _)| a)
                .collect::<Vec<T::AccountId>>();
            ensure!(!present_members.is_empty(), Error::<T>::AttendanceMustBeGreaterThanZero);
            // drain to present members
            let (amt_per_present, remainder_for_publisher) =
                Self::drain_into_accounts(
                    &Self::event_account_id(id),
                    present_members,
                    &publisher
                )?;
            // remove event
            <KickbackEvents<T>>::remove(id);
            <KickbackReservations<T>>::remove_prefix(id);
            // emit event
            Self::deposit_event(
                RawEvent::EventClosed(
                    id,
                    k.reservation_req(),
                    amt_per_present,
                    remainder_for_publisher
                )
            );
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn event_account_id(index: T::KickbackEventId) -> T::AccountId {
        T::EventPool::get().into_sub_account(index)
    }
    pub fn kickback_event_id_is_available(id: T::KickbackEventId) -> bool {
        <KickbackEvents<T>>::get(id).is_none()
    }
    pub fn kickback_event_generate_uid() -> T::KickbackEventId {
        let mut id_counter = <KickbackEventNonce<T>>::get() + 1u32.into();
        while !Self::kickback_event_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <KickbackEventNonce<T>>::put(id_counter);
        id_counter
    }
}

impl<T: Trait> Module<T> {
    /// Drains remaining balance into passed in vector of accounts
    /// -> input vector of accounts must have no duplicates
    pub fn drain_into_accounts(
        from: &T::AccountId,
        accounts: Vec<T::AccountId>,
        remainder_recipient: &T::AccountId,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
        let mut total = T::Currency::total_balance(from);
        let num_of_accounts: u32 = accounts.len() as u32;
        if num_of_accounts == 1 {
            T::Currency::transfer(
                from,
                &accounts[0],
                total,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok((total, BalanceOf::<T>::zero()))
        } else {
            let equal_amt =
                Permill::from_rational_approximation(1u32, num_of_accounts)
                    .mul_floor(total);
            for acc in accounts.iter() {
                T::Currency::transfer(
                    from,
                    &acc,
                    equal_amt,
                    ExistenceRequirement::AllowDeath,
                )?;
                total -= equal_amt;
            }
            // send remainder
            T::Currency::transfer(
                from,
                remainder_recipient,
                total,
                ExistenceRequirement::AllowDeath,
            )?;
            Ok((equal_amt, total))
        }
    }
}
