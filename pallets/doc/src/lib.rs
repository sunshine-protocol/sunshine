#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! To store and track CIDs. This module is NOT intended to be used directly.

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
    DispatchResult,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::doc::FullDoc;

type EncodedObj = Vec<u8>;

pub trait Trait: frame_system::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Encoded object identifier
    type CodeId: Parameter
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

    /// Document identifier
    type DocId: Parameter
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

    /// Content identifier, static ipfs reference type
    type Cid: Parameter + Member + Default;
}

decl_event!(
    pub enum Event<T>
    where
        <T as Trait>::CodeId,
        <T as Trait>::DocId,
        <T as Trait>::Cid,
    {
        NewCodeSet(CodeId),
        NewEncodedObject(CodeId), // TODO: emit EncodedObj after bounded length constraint added
        NewDoc(DocId, Cid),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CodeIdNotRegistered,
        CodeAlreadyRegisteredInSet,
        MustIncludeCodeToCreateNewCodeSet,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Doc {
        /// The nonce for unique code id generation
        CodeIdCounter get(fn code_id_counter): T::CodeId;
        /// The nonce for unique doc id generation
        DocIdCounter get(fn doc_id_counter):T::DocId;

        /// Set of scale-encoded sets
        pub CodeSets get(fn code_sets): double_map
            hasher(blake2_128_concat) T::CodeId,
            hasher(blake2_128_concat) EncodedObj => Option<FullDoc<T::CodeId, EncodedObj>>;

        /// Set of cids
        pub Docs get(fn docs): map
            hasher(blake2_128_concat) T::DocId => Option<FullDoc<T::DocId, T::Cid>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn new_code_set(
            origin,
            initial_codes: Vec<EncodedObj>, // TODO: bound length
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(!initial_codes.is_empty(), Error::<T>::MustIncludeCodeToCreateNewCodeSet);
            let id = Self::generate_code_uid();
            initial_codes.into_iter().for_each(|code| {
                <CodeSets<T>>::insert(id, code.clone(), FullDoc { id, doc: code});
            });
            Self::deposit_event(RawEvent::NewCodeSet(id));
            Ok(())
        }
        #[weight = 0]
        fn new_encoded_object(
            origin,
            id: T::CodeId,
            code: EncodedObj,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(!Self::code_id_dne(id), Error::<T>::CodeIdNotRegistered);
            ensure!(<CodeSets<T>>::get(id, code.clone()).is_none(), Error::<T>::CodeAlreadyRegisteredInSet);
            <CodeSets<T>>::insert(id, code.clone(), FullDoc { id, doc: code});
            Self::deposit_event(RawEvent::NewEncodedObject(id));
            Ok(())
        }
        #[weight = 0]
        fn new_doc(
            origin,
            cid: T::Cid,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let id = Self::generate_doc_uid();
            <Docs<T>>::insert(id, FullDoc { id, doc: cid.clone()});
            Self::deposit_event(RawEvent::NewDoc(id, cid));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn code_id_dne(id: T::CodeId) -> bool {
        <CodeSets<T>>::iter_prefix(id)
            .map(|d| d)
            .collect::<Vec<(EncodedObj, FullDoc<_, _>)>>()
            .is_empty()
    }
    fn generate_code_uid() -> T::CodeId {
        let mut id_counter = <CodeIdCounter<T>>::get() + 1u32.into();
        while !Self::code_id_dne(id_counter) {
            id_counter += 1u32.into();
        }
        <CodeIdCounter<T>>::put(id_counter);
        id_counter
    }
    fn generate_doc_uid() -> T::DocId {
        let mut id_counter = <DocIdCounter<T>>::get() + 1u32.into();
        while <Docs<T>>::get(id_counter).is_some() {
            id_counter += 1u32.into();
        }
        <DocIdCounter<T>>::put(id_counter);
        id_counter
    }
}
