#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The grant module provides structured governance of grant programs

// #[cfg(test)]
// mod tests;

use codec::{
    Codec,
    Encode,
};
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    storage::{
        child,
        IterableStorageMap,
    },
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
    DispatchError,
    DispatchResult,
    ModuleId,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    bounty2::{
        PercentageThreshold,
        ResolutionMetadata,
    },
    grant::{
        ApplicationState,
        Foundation,
        GrantApplication,
        Hasher,
        MilestoneStatus,
        MilestoneSubmission,
    },
    organization::OrgRep,
    traits::{
        GroupMembership,
        OpenThresholdVote,
    },
};

// type aliases
type BalanceOf<T> = <<T as donate::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type FoundationIndex = u32;
type FoundationOf<T> = Foundation<
    <T as org::Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    ResolutionMetadata<
        <T as frame_system::Trait>::AccountId,
        OrgRep<<T as org::Trait>::OrgId>,
        PercentageThreshold<Permill>,
    >,
>;
type ApplicationOf<T> = GrantApplication<
    FoundationIndex,
    <T as org::Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    <T as org::Trait>::OrgId,
    BalanceOf<T>,
    ApplicationState<<T as vote::Trait>::VoteId>,
>;
type MilestoneOf<T> = MilestoneSubmission<
    <T as Trait>::ApplicationId,
    <T as org::Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    <T as org::Trait>::OrgId,
    BalanceOf<T>,
    MilestoneStatus<<T as vote::Trait>::VoteId>,
>;

pub trait Trait:
    frame_system::Trait + org::Trait + vote::Trait + donate::Trait
{
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

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

    /// The foundational foundation
    type Foundation: Get<ModuleId>;

    /// The amount to be held on deposit by the owner of a foundation
    type FoundationDeposit: Get<BalanceOf<Self>>;

    /// The period of time (in blocks) after closing during which
    /// contributors are able to withdraw their funds. After this period, their funds are lost.
    type RetirementPeriod: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
        <T as Trait>::ApplicationId,
        <T as Trait>::MilestoneId,
        Balance = BalanceOf<T>,
    {
        FoundationOpened(FoundationIndex, Balance, IpfsReference),
        GrantApplicationSubmitted(FoundationIndex, ApplicationId, AccountId, Option<OrgId>, Balance, IpfsReference),
        MilestoneSubmitted(ApplicationId, MilestoneId, AccountId, Option<OrgId>, Balance, IpfsReference),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Foundation Does Not Exist
        FoundationDNE,
        ApplicationDNE,
        MilestoneDNE,
        NotAuthorizedToApplyForOutsideTeam,
        NotAuthorizedToSubmitMilestone,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Grant {
        /// Uid generation helper for FoundationIndex
        FoundationNonce get(fn foundation_nonce): FoundationIndex;

        /// Uid generation helpers for ApplicationId
        ApplicationNonce get(fn application_nonce): T::ApplicationId;

        /// Uid generation helpers for MilestoneId
        MilestoneNonce get(fn milestone_nonce): T::MilestoneId;

        // Foundations
        pub Foundations get(fn foundations): map
            hasher(blake2_128_concat) FoundationIndex => Option<FoundationOf<T>>;

        /// Total number of open foundations
        pub FoundationCounter get(fn foundation_counter): u32;

        // Applications
        pub Applications get(fn applications): map
            hasher(blake2_128_concat) T::ApplicationId => Option<ApplicationOf<T>>;

        // Milestones
        pub Milestones get(fn milestones): map
            hasher(blake2_128_concat) T::MilestoneId => Option<MilestoneOf<T>>;

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
        fn open_foundation(
            origin,
            info: T::IpfsReference,
            raise_contribution: Option<BalanceOf<T>>,
            gov: ResolutionMetadata<
                T::AccountId,
                OrgRep<T::OrgId>,
                PercentageThreshold<Permill>,
            >,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;
            let min_deposit = T::FoundationDeposit::get();
            let (initial_deposit, raised) = if let Some(c) = raise_contribution {
                (min_deposit + c, c)
            } else { (min_deposit, BalanceOf::<T>::zero() ) };
            let imb = <T as donate::Trait>::Currency::withdraw(
                &depositer,
                initial_deposit,
                WithdrawReasons::from(WithdrawReason::Transfer),
                ExistenceRequirement::AllowDeath,
            )?;
            let index = Self::foundation_generate_uid();
            // No fees are paid here if we need to create this account; that's why we don't just use the stock `transfer`.
            <T as donate::Trait>::Currency::resolve_creating(&Self::fund_account_id(index), imb);
            // form new foundation
            let foundation = Foundation::new(info.clone(), depositer, min_deposit, raised, gov);
            // insert new foundation
            <Foundations<T>>::insert(index, foundation);
            FoundationCounter::mutate(|n| *n+=1);
            Self::deposit_event(RawEvent::FoundationOpened(index, initial_deposit, info));
            Ok(())
        }// TODO: fn contribute to foundation
        #[weight = 0]
        fn apply_for_grant(
            origin,
            foundation: FoundationIndex,
            description: T::IpfsReference,
            team: Option<T::OrgId>,
            payment: BalanceOf<T>, // TODO: change to enum with drip
        ) -> DispatchResult {
            let applicant = ensure_signed(origin)?;
            ensure!(!Self::foundation_index_is_available(foundation), Error::<T>::FoundationDNE);
            if let Some(t) = team {
                ensure!(<org::Module<T>>::is_member_of_group(t, &applicant), Error::<T>::NotAuthorizedToApplyForOutsideTeam);
            }
            // could check if request exceeds foundation raised funds here
            let application = GrantApplication::new(foundation, description.clone(), applicant.clone(), team, payment);
            let id = Self::application_generate_uid();
            <Applications<T>>::insert(id, application);
            Self::deposit_event(RawEvent::GrantApplicationSubmitted(foundation, id, applicant, team, payment, description));
            Ok(())
        } // TODO: trigger review -> poll in on_finalize
        #[weight = 0]
        fn submit_milestone(
            origin,
            application: T::ApplicationId,
            submission: T::IpfsReference,
            team: Option<T::OrgId>,
            payment: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let auth = Self::can_submit_milestone(application, &submitter, team)?;
            ensure!(auth, Error::<T>::NotAuthorizedToSubmitMilestone);
            let milestone = MilestoneSubmission::new(application, submission.clone(), submitter.clone(), team, payment);
            let id = Self::milestone_generate_uid();
            <Milestones<T>>::insert(id, milestone);
            Self::deposit_event(RawEvent::MilestoneSubmitted(application, id, submitter, team, payment, submission));
            Ok(())
        } // TODO: trigger review -> poll in on_finalize
        #[weight = 0]
        fn trigger_milestone_review(
            origin,
            milestone_id: T::MilestoneId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            // TODO: extract into separate method
            let milestone = <Milestones<T>>::get(milestone_id).ok_or(Error::<T>::MilestoneDNE)?;
            let app = <Applications<T>>::get(milestone.app_ref()).ok_or(Error::<T>::ApplicationDNE)?;
            let foundation = <Foundations<T>>::get(app.foundation()).ok_or(Error::<T>::FoundationDNE)?;

            Ok(())
        }
    }
}

// UID helper methods
impl<T: Trait> Module<T> {
    fn foundation_index_is_available(id: FoundationIndex) -> bool {
        <Foundations<T>>::get(id).is_none()
    }
    fn foundation_generate_uid() -> FoundationIndex {
        let mut id_counter = <FoundationNonce>::get() + 1u32;
        while !Self::foundation_index_is_available(id_counter) {
            id_counter += 1u32;
        }
        <FoundationNonce>::put(id_counter);
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
    fn milestone_id_is_available(id: T::MilestoneId) -> bool {
        <Milestones<T>>::get(id).is_none()
    }
    fn milestone_generate_uid() -> T::MilestoneId {
        let mut id_counter = <MilestoneNonce<T>>::get() + 1u32.into();
        while !Self::milestone_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <MilestoneNonce<T>>::put(id_counter);
        id_counter
    }
}

// Submission and trigger helper methods
impl<T: Trait> Module<T> {
    fn can_submit_milestone(
        app: T::ApplicationId,
        submitter: &T::AccountId,
        team: Option<T::OrgId>,
    ) -> Result<bool, DispatchError> {
        let application =
            <Applications<T>>::get(app).ok_or(Error::<T>::ApplicationDNE)?;
        // ensure that the application is approved?
        todo!() // use `is_child_org()`
    }
    fn can_trigger_app_review(
        app: T::ApplicationId,
        trigger: &T::AccountId,
    ) -> Result<
        (
            T::IpfsReference,
            T::ApplicationId,
            ApplicationOf<T>,
            FoundationOf<T>,
        ),
        DispatchError,
    > {
        todo!()
    }
    fn can_trigger_milestone_review(
        milestone: T::MilestoneId,
        trigger: &T::AccountId,
    ) -> Result<
        (
            T::IpfsReference,
            T::MilestoneId,
            MilestoneOf<T>,
            FoundationOf<T>,
        ),
        DispatchError,
    > {
        todo!()
    }
    fn dispatch_app_review(
        id: T::ApplicationId,
        app: ApplicationOf<T>,
        foundation: FoundationOf<T>,
    ) -> Result<T::VoteId, DispatchError> {
        // update Foundation
        todo!()
    }
    fn dispatch_milestone_review(
        id: T::MilestoneId,
        mile: MilestoneOf<T>,
        foundation: FoundationOf<T>,
        reviewer: Option<T::OrgId>,
    ) -> Result<T::VoteId, DispatchError> {
        // update and insert
        todo!()
    }
}

// Child trie helper methods
impl<T: Trait> Module<T> {
    /// The account ID of the fund pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn fund_account_id(index: FoundationIndex) -> T::AccountId {
        T::Foundation::get().into_sub_account(index)
    }

    /// Find the ID associated with the fund
    ///
    /// Each fund stores information about its contributors and their contributions in a child trie
    /// This helper function calculates the id of the associated child trie.
    pub fn id_from_index(index: FoundationIndex) -> child::ChildInfo {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"foundaon");
        buf.extend_from_slice(&index.to_le_bytes()[..]);

        child::ChildInfo::new_default(T::Hashing::hash(&buf[..]).as_ref())
    }

    /// Record a contribution in the associated child trie.
    pub fn contribution_put(
        index: FoundationIndex,
        who: &T::AccountId,
        balance: &BalanceOf<T>,
    ) {
        let id = Self::id_from_index(index);
        who.using_encoded(|b| child::put(&id, b, &balance));
    }

    /// Lookup a contribution in the associated child trie.
    pub fn contribution_get(
        index: FoundationIndex,
        who: &T::AccountId,
    ) -> BalanceOf<T> {
        let id = Self::id_from_index(index);
        who.using_encoded(|b| child::get_or_default::<BalanceOf<T>>(&id, b))
    }

    /// Remove a contribution from an associated child trie.
    pub fn contribution_kill(index: FoundationIndex, who: &T::AccountId) {
        let id = Self::id_from_index(index);
        who.using_encoded(|b| child::kill(&id, b));
    }

    /// Remove the entire record of contributions in the associated child trie in a single
    /// storage write.
    pub fn foundation_kill(index: FoundationIndex) {
        let id = Self::id_from_index(index);
        child::kill_storage(&id);
    }
}
