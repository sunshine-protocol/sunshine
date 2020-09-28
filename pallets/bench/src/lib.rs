#![recursion_limit = "256"]
//! # Bench Module
//! This module benchmarks common operations
//!
//! - [`bench::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet benchmarks common operations.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod tests;

use alloc::collections::{
    btree_map::BTreeMap,
    btree_set::BTreeSet,
};
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    Parameter,
};
use frame_system::{
    ensure_signed,
    Trait as System,
};
use orml_utilities::OrderedSet;
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

pub trait Trait: System {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// Unique identifier
    type Id: Parameter
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
}

decl_event!(
    pub enum Event<T>
    where
        <T as Trait>::Id
    {
        OpenedGroup(Id, u32),
        AddedMembers(Id, u32),
        RemovedMembers(Id, u32),
        ClosedGroup(Id),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        SetDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bench {
        IdCounter get(fn id_counter): T::Id;

        pub BMaps get(fn b_maps): map
            hasher(blake2_128_concat) T::Id => Option<BTreeMap<T::AccountId, u32>>;

        pub BSets get(fn b_sets): map
            hasher(blake2_128_concat) T::Id => Option<BTreeSet<T::AccountId>>;

        pub OrderedSets get(fn drips): map
            hasher(blake2_128_concat) T::Id => Option<OrderedSet<T::AccountId>>;

        pub KeySets get(fn key_sets): double_map
            hasher(blake2_128_concat) T::Id,
            hasher(blake2_128_concat) T::AccountId => Option<()>;
    }
}

#[derive(
    sp_runtime::RuntimeDebug,
    Copy,
    Clone,
    PartialEq,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
)]
/// All the set constructions that we're benchmarking in storage
pub enum Set {
    Bmaps,
    Bsets,
    Osets,
    Ksets,
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create(
            origin,
            members: Vec<T::AccountId>,
            ty: Set,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let (id, size) = match ty {
                Set::Bmaps => {
                    let mut map = BTreeMap::new();
                    let mut size = 0u32;
                    members.into_iter().for_each(|m| if let None = map.insert(m, 0u32) { size += 1u32; });
                    let id = Self::generate_id();
                    <BMaps<T>>::insert(id, map);
                    (id, size)
                },
                Set::Bsets => {
                    let mut set = BTreeSet::new();
                    let mut size = 0u32;
                    members.into_iter().for_each(|m| if set.insert(m) { size += 1u32; });
                    let id = Self::generate_id();
                    <BSets<T>>::insert(id, set);
                    (id, size)
                },
                Set::Osets => {
                    let set = OrderedSet::from(members);
                    let size: u32 = set.0.len() as u32;
                    let id = Self::generate_id();
                    <OrderedSets<T>>::insert(id, set);
                    (id, size)
                },
                Set::Ksets => {
                    let mut size = 0u32;
                    let id = Self::generate_id();
                    members.into_iter().for_each(|m| {
                        if None == <KeySets<T>>::get(id, &m) {
                            <KeySets<T>>::insert(id, m, ());
                            size += 1u32;
                        }
                    });
                    (id, size)
                },
            };
            Self::deposit_event(RawEvent::OpenedGroup(id, size));
            Ok(())
        }
        #[weight = 0]
        fn add_mems(
            origin,
            id: T::Id,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut m = members;
            m.dedup();
            let mut size = 0u32;
            let mut set = <OrderedSets<T>>::get(id).ok_or(Error::<T>::SetDNE)?;
            m.into_iter().for_each(|a| if set.insert(a) { size += 1u32; });
            <OrderedSets<T>>::insert(id, set);
            Self::deposit_event(RawEvent::AddedMembers(id, size));
            Ok(())
        }
        #[weight = 0]
        fn remove_mems(
            origin,
            id: T::Id,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut m = members;
            m.dedup();
            let mut size = 0u32;
            let mut set = <OrderedSets<T>>::get(id).ok_or(Error::<T>::SetDNE)?;
            m.into_iter().for_each(|a| if set.remove(&a) { size += 1u32; });
            <OrderedSets<T>>::insert(id, set);
            Self::deposit_event(RawEvent::RemovedMembers(id, size));
            Ok(())
        }
        #[weight = 0]
        fn close(
            origin,
            id: T::Id,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            <OrderedSets<T>>::remove(id);
            Self::deposit_event(RawEvent::ClosedGroup(id));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // Not benchmarked, just for UID generation with two storage maps at once
    pub fn generate_id() -> T::Id {
        let mut counter = <IdCounter<T>>::get() + 1u32.into();
        while <OrderedSets<T>>::get(counter).is_some() {
            counter += 1u32.into()
        }
        <IdCounter<T>>::put(counter);
        counter
    }
}
