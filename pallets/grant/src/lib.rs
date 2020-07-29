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
        Saturating,
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
        GetVoteOutcome,
        GroupMembership,
        OpenThresholdVote,
    },
    vote::VoteOutcome,
};

// type aliases
type BalanceOf<T> = <<T as donate::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type VoteCallData<T> = ResolutionMetadata<
    <T as frame_system::Trait>::AccountId,
    OrgRep<<T as org::Trait>::OrgId>,
    PercentageThreshold<Permill>,
>;
type FoundationIndex = u32;
type FoundationOf<T> = Foundation<
    <T as org::Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    VoteCallData<T>,
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
    FoundationIndex,
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

    /// The minimum outside contribution to a foundation
    type MinContribution: Get<BalanceOf<Self>>;

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
        <T as vote::Trait>::VoteId,
        <T as Trait>::ApplicationId,
        <T as Trait>::MilestoneId,
        Balance = BalanceOf<T>,
    {
        FoundationOpened(FoundationIndex, Balance, IpfsReference),
        // index, who, amount_contributed, total_raised
        FoundationContribution(FoundationIndex, AccountId, Balance, Balance),
        // index, who, amount_revoked, total_raised
        FoundationContributionRevoked(FoundationIndex, AccountId, Balance, Balance),
        FoundationClosed(FoundationIndex),
        ApplicationSubmitted(FoundationIndex, ApplicationId, AccountId, Option<OrgId>, Balance, IpfsReference),
        ApplicationReviewTriggered(ApplicationId, VoteId, IpfsReference),
        MilestoneSubmitted(ApplicationId, MilestoneId, AccountId, Option<OrgId>, Balance, IpfsReference),
        MilestoneReviewTriggered(MilestoneId, VoteId, IpfsReference),
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
        NotAuthorizedToTriggerReview,
        ApplicationMustBeApprovedAndLiveToSubmitMilestone,
        // TODO: relax this requirement once we determine how to prove
        // relationships between teams more efficiently (i.e. if both are children)
        MilestoneMustMatchApplicationTeam,
        ContributionMustExceedModuleMinimum,
        CannotRevokeIfContributionDNE,
        DepositerMustCloseFoundationInsteadOfRevokingContribution,
        OnlyDepositerCanCloseFoundation,
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
            gov: VoteCallData<T>,
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
        }
        #[weight = 0]
        fn contribute_to_foundation(
            origin,
            foundation_id: FoundationIndex,
            amount: BalanceOf<T>
        ) -> DispatchResult {
            let contributor = ensure_signed(origin)?;
            ensure!(amount >= T::MinContribution::get(), Error::<T>::ContributionMustExceedModuleMinimum);
            let foundation = <Foundations<T>>::get(foundation_id).ok_or(Error::<T>::FoundationDNE)?;
            T::Currency::transfer(
                &contributor,
                &Self::fund_account_id(foundation_id),
                amount,
                ExistenceRequirement::KeepAlive
            )?;
            let new_foundation = foundation.add_raised(amount);
            let new_raised = new_foundation.raised();
            <Foundations<T>>::insert(foundation_id, new_foundation);
            let balance = Self::contribution_get(foundation_id, &contributor);
            let balance = balance.saturating_add(amount);
            Self::contribution_put(foundation_id, &contributor, &amount);
            Self::deposit_event(RawEvent::FoundationContribution(foundation_id, contributor, balance, new_raised));
            Ok(())
        }
        #[weight = 0]
        fn revoke_contribution_from_foundation(
            origin,
            foundation_id: FoundationIndex,
        ) -> DispatchResult {
            let revoker = ensure_signed(origin)?;
            let foundation = <Foundations<T>>::get(foundation_id).ok_or(Error::<T>::FoundationDNE)?;
            ensure!(revoker != foundation.depositer(), Error::<T>::DepositerMustCloseFoundationInsteadOfRevokingContribution);
            let balance = Self::contribution_get(foundation_id, &revoker);
            ensure!(balance > BalanceOf::<T>::zero(), Error::<T>::CannotRevokeIfContributionDNE);
            T::Currency::transfer(
                &Self::fund_account_id(foundation_id),
                &revoker,
                balance,
                ExistenceRequirement::KeepAlive
            )?;
            let new_foundation = foundation.subtract_raised(balance);
            let new_raise = new_foundation.raised();
            Self::contribution_kill(foundation_id, &revoker);
            Self::deposit_event(RawEvent::FoundationContributionRevoked(foundation_id, revoker, balance, new_raise));
            Ok(())
        }
        #[weight = 0]
        fn close_foundation(
            origin,
            id: FoundationIndex,
            donate_to: OrgRep<T::OrgId>,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;
            let foundation = <Foundations<T>>::get(id).ok_or(Error::<T>::FoundationDNE)?;
            ensure!(depositer == foundation.depositer(), Error::<T>::OnlyDepositerCanCloseFoundation);
            let sender = Self::fund_account_id(id);
            let remainder = <donate::Module<T>>::donate(&sender, donate_to, foundation.raised())?;
            // return deposit + remainder to depositer
            // <problem here if previous call succeeds and this call fails/>
            T::Currency::transfer(
                &sender,
                &depositer,
                foundation.deposit() + remainder,
                ExistenceRequirement::KeepAlive,
            )?;
            // kill child trie
            Self::foundation_kill(id);
            // remove all storage items related to this foundation
            Self::recursive_remove_foundation(id);
            Self::deposit_event(RawEvent::FoundationClosed(id));
            Ok(())
        }
        #[weight = 0]
        fn apply_for_grant(
            origin,
            foundation: FoundationIndex,
            description: T::IpfsReference,
            team: Option<T::OrgId>,
            payment: BalanceOf<T>,
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
            Self::deposit_event(RawEvent::ApplicationSubmitted(foundation, id, applicant, team, payment, description));
            Ok(())
        }
        #[weight = 0]
        fn trigger_application_review(
            origin,
            application_id: T::ApplicationId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            let (application, foundation) = Self::can_trigger_app_review(application_id, &trigger)?;
            let new_vote_id = <vote::Module<T>>::open_threshold_vote(Some(application.submission().clone()), foundation.gov().org(), foundation.gov().threshold().pct_to_pass(), foundation.gov().threshold().pct_to_fail(), None)?;
            let new_app = application.set_state(ApplicationState::UnderReviewByAcceptanceCommittee(new_vote_id));
            <Applications<T>>::insert(application_id, new_app);
            Self::deposit_event(RawEvent::ApplicationReviewTriggered(application_id, new_vote_id, application.submission()));
            Ok(())
        }
        #[weight = 0]
        fn submit_milestone(
            origin,
            application: T::ApplicationId,
            submission: T::IpfsReference,
            team: Option<T::OrgId>,
            payment: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let (foundation_id, auth) = Self::can_submit_milestone(application, &submitter, team)?;
            ensure!(auth, Error::<T>::NotAuthorizedToSubmitMilestone);
            let milestone = MilestoneSubmission::new((foundation_id, application), submission.clone(), submitter.clone(), team, payment);
            let id = Self::milestone_generate_uid();
            <Milestones<T>>::insert(id, milestone);
            Self::deposit_event(RawEvent::MilestoneSubmitted(application, id, submitter, team, payment, submission));
            Ok(())
        }
        #[weight = 0]
        fn trigger_milestone_review(
            origin,
            milestone_id: T::MilestoneId,
            supervisor: Option<VoteCallData<T>>,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            let (milestone, foundation) = Self::can_trigger_milestone_review(milestone_id, &trigger)?;
            let review_board = if let Some(s) = supervisor {
                s
            } else { foundation.gov() };
            let new_vote_id = <vote::Module<T>>::open_threshold_vote(Some(milestone.submission().clone()), review_board.org(), review_board.threshold().pct_to_pass(), review_board.threshold().pct_to_fail(), None)?;
            Self::deposit_event(RawEvent::MilestoneReviewTriggered(milestone_id, new_vote_id, milestone.submission()));
            Ok(())
        }
        // TODO: move inner logic into helper methods
        fn on_finalize(_n: T::BlockNumber) {
            if <frame_system::Module<T>>::block_number() % Self::application_poll_frequency() == Zero::zero() {
                let _ = <Applications<T>>::iter()
                    .filter(|(_, app)| app.under_review().is_some())
                    .map(|(id, app)| -> DispatchResult {
                        if let Some(v) = app.under_review() {
                            let status = <vote::Module<T>>::get_vote_outcome(v)?;
                            match status {
                                VoteOutcome::Approved => {
                                    let new_app = app.set_state(ApplicationState::ApprovedAndLive);
                                    <Applications<T>>::insert(id, new_app);
                                }
                                VoteOutcome::Rejected => {
                                    <Applications<T>>::remove(id);
                                }
                                _ => (),
                            }
                        }
                        Ok(())
                    });
            }

            if <frame_system::Module<T>>::block_number() % Self::milestone_poll_frequency() == Zero::zero() {
                let _ = <Milestones<T>>::iter()
                    .filter(|(_, mile)| mile.under_review().is_some())
                    .map(|(id, mile)| -> DispatchResult {
                        if let Some(v) = mile.under_review() {
                            let status = <vote::Module<T>>::get_vote_outcome(v)?;
                            match status {
                                VoteOutcome::Approved => {
                                    let fid = Self::fund_account_id(mile.base_foundation());
                                    if let Some(t) = mile.team() {
                                        let remainder = <donate::Module<T>>::donate(&fid, OrgRep::Weighted(t), mile.payment())?;
                                        if remainder > BalanceOf::<T>::zero() {
                                            T::Currency::transfer(
                                                &fid,
                                                &mile.submitter(),
                                                remainder,
                                                ExistenceRequirement::KeepAlive
                                            )?;
                                        }
                                    } else {
                                        T::Currency::transfer(
                                            &fid,
                                            &mile.submitter(),
                                            mile.payment(),
                                            ExistenceRequirement::KeepAlive
                                        )?;
                                    }
                                    // update foundation based on spend
                                    let foundation = <Foundations<T>>::get(mile.base_foundation()).ok_or(Error::<T>::FoundationDNE)?;
                                    let new_foundation = foundation.subtract_raised(mile.payment());
                                    <Foundations<T>>::insert(mile.base_foundation(), new_foundation);
                                    // update milestone
                                    let new_mile = mile.set_state(MilestoneStatus::ApprovedAndTransferExecuted);
                                    <Milestones<T>>::insert(id, new_mile);
                                }
                                VoteOutcome::Rejected => {
                                    <Milestones<T>>::remove(id);
                                }
                                _ => (),
                            }
                        }
                        Ok(())
                    });
            }
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

// Permissions
impl<T: Trait> Module<T> {
    fn can_submit_milestone(
        app: T::ApplicationId,
        submitter: &T::AccountId,
        team: Option<T::OrgId>,
    ) -> Result<(FoundationIndex, bool), DispatchError> {
        let application =
            <Applications<T>>::get(app).ok_or(Error::<T>::ApplicationDNE)?;
        ensure!(
            application.approved_and_live(),
            Error::<T>::ApplicationMustBeApprovedAndLiveToSubmitMilestone
        );
        // TODO: relax strict equality for some provable relation (co-children)
        ensure!(
            application.team() == team,
            Error::<T>::MilestoneMustMatchApplicationTeam
        );
        let auth = if let Some(t) = team {
            <org::Module<T>>::is_member_of_group(t, submitter)
                || application.is_submitter(submitter)
        } else {
            application.is_submitter(submitter)
        };
        Ok((application.foundation(), auth))
    }
    fn can_trigger_app_review(
        app: T::ApplicationId,
        trigger: &T::AccountId,
    ) -> Result<(ApplicationOf<T>, FoundationOf<T>), DispatchError> {
        let application =
            <Applications<T>>::get(app).ok_or(Error::<T>::ApplicationDNE)?;
        let foundation = <Foundations<T>>::get(application.foundation())
            .ok_or(Error::<T>::FoundationDNE)?;
        let auth = foundation.gov().is_sudo(trigger)
            || <org::Module<T>>::is_member_of_group(
                foundation.gov().org().org(),
                trigger,
            );
        ensure!(auth, Error::<T>::NotAuthorizedToTriggerReview);
        Ok((application, foundation))
    }
    fn can_trigger_milestone_review(
        milestone_id: T::MilestoneId,
        trigger: &T::AccountId,
    ) -> Result<(MilestoneOf<T>, FoundationOf<T>), DispatchError> {
        let milestone = <Milestones<T>>::get(milestone_id)
            .ok_or(Error::<T>::MilestoneDNE)?;
        let app = <Applications<T>>::get(milestone.base_application())
            .ok_or(Error::<T>::ApplicationDNE)?;
        let foundation = <Foundations<T>>::get(app.foundation())
            .ok_or(Error::<T>::FoundationDNE)?;
        let auth = foundation.gov().is_sudo(trigger)
            || <org::Module<T>>::is_member_of_group(
                foundation.gov().org().org(),
                trigger,
            );
        ensure!(auth, Error::<T>::NotAuthorizedToTriggerReview);
        Ok((milestone, foundation))
    }
}

// Storage helper for removing all storage items associated with a foundation
impl<T: Trait> Module<T> {
    pub fn recursive_remove_foundation(id: FoundationIndex) {
        <Foundations<T>>::remove(id);
        FoundationCounter::mutate(|n| *n -= 1);
        <Applications<T>>::iter()
            .filter(|(_, app)| app.foundation() == id)
            .for_each(|(i, _)| <Applications<T>>::remove(i));
        <Milestones<T>>::iter()
            .filter(|(_, mile)| mile.base_foundation() == id)
            .for_each(|(i, _)| <Milestones<T>>::remove(i));
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
