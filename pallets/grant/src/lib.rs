#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Grants module

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
        WithdrawReason,
        WithdrawReasons,
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
    DispatchResult,
    ModuleId,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    grant::{
        ApplicationState,
        Foundation,
        GrantApplication,
        MilestoneStatus,
        MilestoneSubmission,
        Recipient,
    },
    meta::{
        ResolutionMetadata,
        VoteMetadata,
    },
    organization::OrgRep,
};

// type aliases
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type GovernanceOf<T> = ResolutionMetadata<
    <T as frame_system::Trait>::AccountId,
    VoteMetadata<
        OrgRep<<T as org::Trait>::OrgId>,
        <T as vote::Trait>::Signal,
        Permill,
        <T as frame_system::Trait>::BlockNumber,
    >,
>;
type FoundationOf<T> =
    Foundation<<T as org::Trait>::IpfsReference, BalanceOf<T>, GovernanceOf<T>>;
type RecipientOf<T> =
    Recipient<<T as frame_system::Trait>::AccountId, <T as org::Trait>::OrgId>;
type GrantApp<T> = GrantApplication<
    <T as Trait>::FoundationId,
    <T as org::Trait>::IpfsReference,
    RecipientOf<T>,
    BalanceOf<T>,
    ApplicationState<<T as vote::Trait>::VoteId>,
>;
type Milestone<T> = MilestoneSubmission<
    <T as Trait>::FoundationId,
    <T as Trait>::ApplicationId,
    <T as org::Trait>::IpfsReference,
    RecipientOf<T>,
    BalanceOf<T>,
    MilestoneStatus<<T as vote::Trait>::VoteId>,
>;

pub trait Trait: frame_system::Trait + org::Trait + vote::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The foundation identifier
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

    /// The application identifier
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
        + Zero;

    /// The milestone identifier
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
        + Zero;

    /// The foundational foundation
    type BigFoundation: Get<ModuleId>;

    /// Minimum deposit to create foundation
    type MinDeposit: Get<BalanceOf<Self>>;

    /// Minimum contribution to open foundation
    type MinContribution: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::IpfsReference,
        <T as Trait>::FoundationId,
        <T as Trait>::ApplicationId,
        <T as Trait>::MilestoneId,
        Balance = BalanceOf<T>,
        Recipient = RecipientOf<T>,
    {
        FoundationCreated(FoundationId, Balance, IpfsReference),
        FoundationDonation(AccountId, Balance, FoundationId, Balance),
        ApplicationSubmitted(FoundationId, ApplicationId, Recipient, Balance, IpfsReference),
        MilestoneSubmitted(FoundationId, ApplicationId, MilestoneId, Recipient, Balance, IpfsReference),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Foundation Does Not Exist
        FoundationDNE,
        ApplicationDNE,
        DepositBelowMinDeposit,
        ContributionBelowMinContribution,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bounty {
        /// Uid generation helper for FoundationId
        FoundationNonce get(fn foundation_nonce): T::FoundationId;

        /// Uid generation helpers for ApplicationId
        ApplicationNonce get(fn application_nonce): T::ApplicationId;

        /// Uid generation helper for MilestoneIds
        MilestoneNonce get(fn milestone_nonce): map
            hasher(blake2_128_concat) T::ApplicationId => T::MilestoneId;

        /// Foundations
        pub Foundations get(fn foundations): map
            hasher(blake2_128_concat) T::FoundationId => Option<FoundationOf<T>>;
        /// History of Foundation Inflows
        pub FoundationDonations get(fn bounty_tips): double_map
            hasher(blake2_128_concat) T::FoundationId,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;

        /// Applications
        pub Applications get(fn applications): map
            hasher(blake2_128_concat) T::ApplicationId => Option<GrantApp<T>>;
        /// Milestones
        pub Milestones get(fn milestones): double_map
            hasher(blake2_128_concat) T::ApplicationId,
            hasher(blake2_128_concat) T::MilestoneId => Option<Milestone<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_foundation(
            origin,
            info: T::IpfsReference,
            amount: BalanceOf<T>,
            governance: GovernanceOf<T>,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;
            ensure!(amount >= T::MinDeposit::get(), Error::<T>::DepositBelowMinDeposit);
            let imb = T::Currency::withdraw(
                &depositer,
                amount,
                WithdrawReasons::from(WithdrawReason::Transfer),
                ExistenceRequirement::AllowDeath,
            )?;
            let foundation = FoundationOf::<T>::new(info.clone(), amount, governance);
            let id = Self::foundation_generate_uid();
            T::Currency::resolve_creating(&Self::foundation_account_id(id), imb);
            <Foundations<T>>::insert(id, foundation);
            <FoundationDonations<T>>::insert(id, &depositer, amount);
            Self::deposit_event(RawEvent::FoundationCreated(id, amount, info));
            Ok(())
        }
        #[weight = 0]
        fn donate_to_foundation(
            origin,
            id: T::FoundationId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let giver = ensure_signed(origin)?;
            ensure!(amount >= T::MinContribution::get(), Error::<T>::ContributionBelowMinContribution);
            let foundation = <Foundations<T>>::get(id).ok_or(Error::<T>::FoundationDNE)?;
            T::Currency::transfer(
                &giver,
                &Self::foundation_account_id(id),
                amount,
                ExistenceRequirement::KeepAlive,
            )?;
            let new_amount = if let Some(a) = <FoundationDonations<T>>::get(id, &giver) {
                a + amount
            } else { amount };
            let new_foundation = foundation.add_funds(amount);
            let total = new_foundation.funds();
            <Foundations<T>>::insert(id, new_foundation);
            <FoundationDonations<T>>::insert(id, &giver, new_amount);
            Self::deposit_event(RawEvent::FoundationDonation(giver, new_amount, id, total));
            Ok(())
        }
        #[weight = 0]
        fn submit_application(
            origin,
            foundation_id: T::FoundationId,
            submission_ref: T::IpfsReference,
            recipient: RecipientOf<T>,
            amount_requested: BalanceOf<T>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(!Self::foundation_id_is_available(foundation_id), Error::<T>::FoundationDNE);
            let app = GrantApp::<T>::new(foundation_id, submission_ref.clone(), recipient.clone(), amount_requested);
            let id = Self::application_generate_uid();
            <Applications<T>>::insert(id, app);
            Self::deposit_event(RawEvent::ApplicationSubmitted(foundation_id, id, recipient, amount_requested, submission_ref));
            Ok(())
        }
        #[weight = 0]
        fn submit_milestone(
            origin,
            foundation_id: T::FoundationId,
            application_id: T::ApplicationId,
            submission_ref: T::IpfsReference,
            recipient: RecipientOf<T>,
            amount_requested: BalanceOf<T>,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            ensure!(!Self::foundation_id_is_available(foundation_id), Error::<T>::FoundationDNE);
            ensure!(!Self::application_id_is_available(application_id), Error::<T>::ApplicationDNE);
            let milestone = Milestone::<T>::new((foundation_id, application_id), submission_ref.clone(), recipient.clone(), amount_requested);
            let id = Self::milestone_generate_uid(application_id);
            <Milestones<T>>::insert(application_id, id, milestone);
            Self::deposit_event(RawEvent::MilestoneSubmitted(foundation_id, application_id, id, recipient, amount_requested, submission_ref));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn foundation_account_id(index: T::FoundationId) -> T::AccountId {
        T::BigFoundation::get().into_sub_account(index)
    }
    fn foundation_id_is_available(id: T::FoundationId) -> bool {
        <Foundations<T>>::get(id).is_none()
    }
    fn foundation_generate_uid() -> T::FoundationId {
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
    fn application_generate_uid() -> T::ApplicationId {
        let mut id_counter = <ApplicationNonce<T>>::get() + 1u32.into();
        while !Self::application_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <ApplicationNonce<T>>::put(id_counter);
        id_counter
    }
    fn milestone_id_is_available(
        application: T::ApplicationId,
        id: T::MilestoneId,
    ) -> bool {
        <Milestones<T>>::get(application, id).is_none()
    }
    fn milestone_generate_uid(app_id: T::ApplicationId) -> T::MilestoneId {
        let mut id_counter = <MilestoneNonce<T>>::get(app_id) + 1u32.into();
        while !Self::milestone_id_is_available(app_id, id_counter) {
            id_counter += 1u32.into();
        }
        <MilestoneNonce<T>>::insert(app_id, id_counter);
        id_counter
    }
    fn _recursive_remove_foundation(id: T::FoundationId) {
        <Foundations<T>>::remove(id);
        <Applications<T>>::iter()
            .filter(|(_, app)| app.foundation_id() == id)
            .for_each(|(app_id, _)| {
                <Applications<T>>::remove(app_id);
                <Milestones<T>>::remove_prefix(app_id);
            });
    }
}
