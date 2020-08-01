#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Metagovernance

// #[cfg(test)]
// mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    traits::{
        ExistenceRequirement,
        Get,
    },
    Parameter,
};
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchError,
    DispatchResult,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};

pub trait Trait: frame_system::Trait + org::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid type
    type IpfsReference: Parameter + Member + Default;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,

    {
        PlaceHolderEvent(AccountId, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Must register dispute with resolution path before raising one
        PlaceholderError,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Meta {
        /// The placeholder
        pub PlaceholderStorage get(fn placeholder_storage): u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn placeholder(
            origin,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
    }
}
