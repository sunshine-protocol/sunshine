#![recursion_limit = "256"]
//! # Court Module
//! This module expresses hierarchical dispute resolution. It stores a sequence of vote metadata
//! to schedule and dispatch votes to resolve two-party disputes when they arise.
//!
//! - [`court::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet stores a sequence of vote metadata to schedule and dispatch votes
//! for dispute resolution (upon trigger of either party).
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html
#![cfg_attr(not(feature = "std"), no_std)]

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
        Get,
        ReservableCurrency,
    },
    Parameter,
};
use frame_system::{
    ensure_signed,
    Trait as System,
};
use org::Trait as Org;
use parity_scale_codec::Codec;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
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
use util::court::{
    Court,
    Threshold,
};
use vote::Trait as Vote;

/// The balances type for this module
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as System>::AccountId>>::Balance;
type ThresholdOf<T> = Threshold<<T as Trait>::RankId, <T as Vote>::ThresholdId>;
type CourtOf<T> = Court<
    <T as Trait>::CourtId,
    <T as System>::AccountId,
    BalanceOf<T>,
    ThresholdOf<T>,
>;
pub trait Trait: System + Org + Vote {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// The currency type
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The identifier for courts registered on-chain
    type CourtId: Parameter
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

    /// The rank identifier for ordering vote metadata
    type RankId: Parameter
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

    /// Minimum bond for any court registered on-chain
    type MinBond: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as Org>::OrgId,
        <T as Vote>::VoteId,
        <T as Trait>::CourtId,
        Balance = BalanceOf<T>,

    {
        NewCourtSeq(CourtId, Balance),
        VoteDispatched(CourtId, OrgId, VoteId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Court Does Not Exist
        CourtDNE,
        BondMustExceedMin,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        /// The nonce for unique court id generation
        CourtIdCounter get(fn court_id_counter): T::CourtId;

        /// The number of open courts
        pub CourtCount get(fn count): u32;

        /// The state of courts
        pub Courts get(fn courts): map
            hasher(blake2_128_concat) T::CourtId => Option<CourtOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_court_seq(
            origin,
            controller: Option<T::AccountId>,
            bond: BalanceOf<T>,
            vote_seq: Vec<T::ThresholdId>,
        ) -> DispatchResult {
            ensure!(bond >= T::MinBond::get(), Error::<T>::BondMustExceedMin);
            let _ = ensure_signed(origin)?;
            let id = Self::generate_court_uid();
            let court = CourtOf::<T>::new(id, controller, bond, Self::vote_thresholds(&vote_seq.as_slice()));
            <Courts<T>>::insert(id, court);
            Self::deposit_event(RawEvent::NewCourtSeq(id, bond));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn vote_thresholds(from: &[T::ThresholdId]) -> Vec<ThresholdOf<T>> {
        let mut counter: T::RankId = Zero::zero();
        from.to_vec()
            .into_iter()
            .map(|id| {
                let ret = ThresholdOf::<T>::new(counter, id);
                counter += 1u32.into();
                ret
            })
            .collect::<Vec<ThresholdOf<T>>>()
    }
    pub fn generate_court_uid() -> T::CourtId {
        let mut count = <CourtIdCounter<T>>::get() + 1u32.into();
        while <Courts<T>>::get(count).is_some() {
            count += 1u32.into();
        }
        <CourtIdCounter<T>>::put(count);
        count
    }
}
