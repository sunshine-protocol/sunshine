#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The bounty module allows any `AccountId` to post bounties with rules for approval

// #[cfg(test)]
// mod tests;

use codec::Codec;
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
        Get,
        ReservableCurrency,
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
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    organization::OrgRep,
    traits::{
        bounty2::{
            PostBounty,
            SubmitForBounty,
        },
        GroupMembership,
        IDIsAvailable,
        OpenThresholdVote,
    },
};

/// The balances type for this module
type BalanceOf<T> = <<T as donate::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + org::Trait + vote::Trait + donate::Trait
{
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The grant foundation post identifier
    type FoundationId: Parameter
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

    /// The grant application identifier
    type ApplicationId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero; // + Into<Self::FoundationId>

    /// The grant milestone identifier
    type MilestoneId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero; // + Into<Self::MilestoneId>
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::BlockNumber,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
    {
        FoundationCreated(),
        GrantOpportunityPosted(AccountId, OrgId, BlockNumber, Option<AccountId>, IpfsReference),
        GrantApplicationSubmitted(),
        GrantApplicationReviewTriggered(),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Foundation Does Not Exist
        FoundationDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Grant {
        /// Uid generation helper for FoundationId
        FoundationNonce get(fn foundation_nonce): T::FoundationId;

        /// Uid generation helpers for ApplicationId
        ApplicationNonce get(fn application_nonce): T::ApplicationId;

        /// Uid generation helpers for MilestoneId
        MilestoneNonce get(fn milestone_nonce): T::MilestoneId;

        // Foundations
        pub Foundations get(fn foundations): map
            hasher(blake2_128_concat) T::FoundationId => Option<
                T::FoundationId // todo!()
            >;

        // Applications
        pub Applications get(fn applications): map
            hasher(blake2_128_concat) T::ApplicationId => Option<
                T::ApplicationId // todo!()
            >;

        // Milestones
        pub Milestones get(fn milestones): map
            hasher(blake2_128_concat) T::MilestoneId => Option<
                T::MilestoneId // todo!()
            >;

        /// Frequency with which applications are polled and dealt with
        pub ApplicationPollFrequency get(fn application_poll_frequency) config(): T::BlockNumber;
        /// Frequency with which milestone submissions are polled and dealt with
        pub MilestonePollFrequency get(fn milestone_poll_frequency) config(): T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_foundation(
            origin,
            _info: T::IpfsReference,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn foundation_id_is_available(id: T::FoundationId) -> bool {
        <Foundations<T>>::get(id).is_none()
    }
    fn foundation_generate_unique_id() -> T::FoundationId {
        let mut id_counter = <FoundationNonce<T>>::get() + 1u32.into();
        while !Self::foundation_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <FoundationNonce<T>>::put(id_counter);
        id_counter
    }
    fn application_id_is_available(id: T::ApplicationId) -> bool {
        <Applications<T>>::get(id).is_none()
    }
    fn application_generate_unique_id() -> T::ApplicationId {
        let mut id_counter = <ApplicationNonce<T>>::get() + 1u32.into();
        while !Self::application_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <ApplicationNonce<T>>::put(id_counter);
        id_counter
    }
    fn milestone_id_is_available(id: T::MilestoneId) -> bool {
        <Milestones<T>>::get(id).is_none()
    }
    fn milestone_generate_unique_id() -> T::MilestoneId {
        let mut id_counter = <MilestoneNonce<T>>::get() + 1u32.into();
        while !Self::milestone_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <MilestoneNonce<T>>::put(id_counter);
        id_counter
    }
}
