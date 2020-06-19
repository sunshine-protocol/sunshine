#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The court module is for enforcing hierarchical dispute resolution in governance

use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_system::{self as system, ensure_signed};
use sp_runtime::DispatchResult;
use sp_std::prelude::*;

// - I expect to build a wrapper around vote-yesno here that adds veto rights and is simply count_threshold_voting
// such that the supervisors are the electorate
// - how are supervisors changed? give that control to an organizational supervisor now

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId
    {
        PlaceHolder(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PlaceHolderError,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        pub PlaceHolderStorageValue get(fn place_holder_storage_value): u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn fake_method(origin) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            if PlaceHolderStorageValue::get() == 69u32 {
                return Err(Error::<T>::PlaceHolderError.into());
            }
            Self::deposit_event(RawEvent::PlaceHolder(signer));
            Ok(())
        }
    }
}
