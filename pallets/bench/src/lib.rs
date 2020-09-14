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
use orml_utilities::OrderedSet;
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

pub trait Trait: System {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// The identifier
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
    trait Store for Module<T: Trait> as Drip {
        IdCounter get(fn id_counter): T::Id;

        pub OrderedSets get(fn drips): map
            hasher(blake2_128_concat) T::Id => Option<OrderedSet<T::AccountId>>;

        pub KeySets get(fn key_sets): double_map
            hasher(blake2_128_concat) T::Id,
            hasher(blake2_128_concat) T::AccountId => Option<()>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_os(
            origin,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let id = Self::generate_id();
            let set = OrderedSet::from(members);
            let size: u32 = set.0.len() as u32;
            <OrderedSets<T>>::insert(id, set);
            Self::deposit_event(RawEvent::OpenedGroup(id, size));
            Ok(())
        }
        #[weight = 0]
        fn create_ks(
            origin,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let id = Self::generate_id();
            let mut m = members;
            m.dedup();
            let size: u32 = m.len() as u32;
            m.into_iter().for_each(|a| {
                <KeySets<T>>::insert(id, a, ());
            });
            Self::deposit_event(RawEvent::OpenedGroup(id, size));
            Ok(())
        }
        #[weight = 0]
        fn add_mems_os(
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
        fn add_mems_ks(
            origin,
            id: T::Id,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut m = members;
            m.dedup();
            let mut size = 0u32;
            m.into_iter().for_each(|a| {
                if <KeySets<T>>::get(id, &a).is_none() {
                    <KeySets<T>>::insert(id, a, ());
                    size += 1u32;
                }
            });
            Self::deposit_event(RawEvent::AddedMembers(id, size));
            Ok(())
        }
        #[weight = 0]
        fn remove_mems_os(
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
        fn remove_mems_ks(
            origin,
            id: T::Id,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let mut m = members;
            m.dedup();
            let mut size = 0u32;
            m.into_iter().for_each(|a| {
                if <KeySets<T>>::get(id, &a).is_some() {
                    <KeySets<T>>::remove(id, a);
                    size += 1u32;
                }
            });
            Self::deposit_event(RawEvent::RemovedMembers(id, size));
            Ok(())
        }
        #[weight = 0]
        fn close_os(
            origin,
            id: T::Id,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            <OrderedSets<T>>::remove(id);
            Self::deposit_event(RawEvent::ClosedGroup(id));
            Ok(())
        }
        #[weight = 0]
        fn close_ks(
            origin,
            id: T::Id,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            <KeySets<T>>::remove_prefix(id);
            Self::deposit_event(RawEvent::ClosedGroup(id));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // Not benchmarked, just for UID generation with two storage maps at once
    pub fn generate_id() -> T::Id {
        let mut counter = <IdCounter<T>>::get() + 1u32.into();
        while <OrderedSets<T>>::get(counter).is_some()
            || !<KeySets<T>>::iter_prefix(counter)
                .into_iter()
                .map(|(a, _)| a)
                .collect::<Vec<T::AccountId>>()
                .is_empty()
        {
            counter += 1u32.into()
        }
        <IdCounter<T>>::put(counter);
        counter
    }
}
