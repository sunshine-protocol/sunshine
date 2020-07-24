#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The bounty module allows registered organizations with on-chain bank accounts to
//! register as a foundation to post bounties and supervise ongoing grant pursuits.

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        ReservableCurrency,
    },
    Parameter,
};
use frame_system::{
    self as system,
    ensure_signed,
};
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
    bank::{
        BankOrAccount,
        OnChainTreasuryID,
    },
    bounty::{
        ApplicationState,
        BountyInformation,
        BountyMapID,
        GrantApplication,
        MilestoneStatus,
        MilestoneSubmission,
    },
    court::ResolutionMetadata,
    organization::OrgRep,
    traits::{
        ApproveGrant,
        ApproveWithoutTransfer,
        GenerateUniqueID,
        GetVoteOutcome,
        GroupMembership,
        IDIsAvailable,
        OpenVote,
        OrganizationSupervisorPermissions,
        PostBounty,
        ReturnsBountyIdentifier,
        SeededGenerateUniqueID,
        StartReview,
        SubmitGrantApplication,
        SubmitMilestone,
        SuperviseGrantApplication,
    },
    vote::VoteOutcome,
};

/// The balances type for this module is inherited from bank
pub type BalanceOf<T> = <<T as bank::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + org::Trait + vote::Trait + bank::Trait
{
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The bounty identifier in this module
    type BountyId: Parameter
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

    /// Unambiguous lower bound for bounties posted with this module
    type BountyLowerBound: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::IpfsReference,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BountyId,
        Balance = BalanceOf<T>,
    {
        BountyPosted(BountyId, AccountId, Balance, IpfsReference),
        BountyApplicationSubmitted(BountyId, BountyId, AccountId, Option<OnChainTreasuryID>, Balance),
        SudoApprovedBountyApplication(AccountId, BountyId, BountyId, ApplicationState<VoteId>),
        ApplicationReviewTriggered(AccountId, BountyId, BountyId, ApplicationState<VoteId>),
        ApplicationPolled(AccountId, BountyId, BountyId, ApplicationState<VoteId>),
        MilestoneSubmitted(AccountId, BountyId, BountyId, BountyId, Balance, IpfsReference),
        MilestoneReviewTriggered(AccountId, BountyId, BountyId, MilestoneStatus<VoteId>),
        SudoApprovedMilestone(AccountId, BountyId, BountyId, MilestoneStatus<VoteId>),
        MilestonePolled(AccountId, BountyId, BountyId, MilestoneStatus<VoteId>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotPostBountyIfBankReferencedDNE,
        CannotPostBountyOnBehalfOfOrgWithInvalidTransferReference,
        CannotPostBountyOnBehalfOfOrgWithInvalidSpendReservation,
        CannotPostBountyIfAmountExceedsAmountLeftFromSpendReference,
        GrantApplicationRequestExceedsBountyFundingReserved,
        GrantApplicationFailsForBountyThatDNE,
        CannotApplyForBountyWithOrgBankAccountThatDNE,
        SubmitterNotAuthorizedToSubmitGrantAppForOrg,
        CannotReviewApplicationIfBountyDNE,
        CannotReviewApplicationIfApplicationDNE,
        ApplicationMustBeSubmittedAwaitingResponseToTriggerReview,
        CannotSudoApproveIfBountyDNE,
        CannotSudoApproveAppIfNotAssignedSudo,
        CannotSudoApproveIfGrantAppDNE,
        AppStateCannotBeSudoApprovedForAGrantFromCurrentState,
        CannotPollApplicationIfBountyDNE,
        CannotPollApplicationIfApplicationDNE,
        CannotSubmitMilestoneIfBaseBountyDNE,
        CannotSubmitMilestoneIfApplicationDNE,
        ApplicationMustBeApprovedToSubmitMilestones,
        InvalidBankReferenceInApplicationThrownInMilestoneSubmission,
        MilestoneSubmissionNotAuthorizedBySubmitterForBankOrgApplication,
        MilestoneSubmissionNotAuthorizedBySubmitterForIndividualApplication,
        CannotTriggerMilestoneReviewIfBaseBountyDNE,
        CannotTriggerMilestoneReviewIfSubmissionDNE,
        CannotTriggerMilestoneReviewIfSubmissionNotAwaitingResponseAkaWrongState,
        CannotSudoApproveMilestoneIfBaseBountyDNE,
        CallerNotAuthorizedToSudoApproveMilestone,
        CannotSudoApproveMilestoneIfBaseAppDNE,
        CannotSudoApproveMilestoneThatDNE,
        MilestoneCannotBeSudoApprovedFromTheCurrentState,
        CannotPollMilestoneThatDNE,
        CannotPollMilestoneIfBaseAppDNE,
        CannotPollMilestoneSubmissionIfBaseBountyDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bounty {
        /// Uid generation helper for main BountyId
        BountyNonce get(fn bounty_nonce): T::BountyId;

        /// Uid generation helpers for second keys on auxiliary maps
        BountyAssociatedNonces get(fn bounty_associated_nonces): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) BountyMapID => T::BountyId;

        /// Posted bounty details
        pub LiveBounties get(fn foundation_sponsored_bounties): map
            hasher(opaque_blake2_256) T::BountyId => Option<
                BountyInformation<
                    BankOrAccount<
                        OnChainTreasuryID,
                        T::AccountId
                    >,
                    T::IpfsReference,
                    BalanceOf<T>,
                    ResolutionMetadata<
                        OrgRep<T::OrgId>,
                        T::Signal,
                        T::BlockNumber,
                    >,
                >
            >;

        /// All bounty applications
        pub BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<
                GrantApplication<
                    T::AccountId,
                    OnChainTreasuryID,
                    BalanceOf<T>,
                    T::IpfsReference,
                    ApplicationState<T::VoteId>,
                >
            >;

        /// All milestone submissions
        pub MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<
                MilestoneSubmission<
                    T::AccountId,
                    T::BountyId,
                    T::IpfsReference,
                    BalanceOf<T>,
                    MilestoneStatus<T::VoteId>
                >
            >;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn account_posts_bounty(
            origin,
            description: T::IpfsReference,
            amount_reserved_for_bounty: BalanceOf<T>,
            acceptance_committee: ResolutionMetadata<
                OrgRep<T::OrgId>,
                T::Signal,
                T::BlockNumber,
            >,
            supervision_committee: Option<
                ResolutionMetadata<
                    OrgRep<T::OrgId>,
                    T::Signal,
                    T::BlockNumber,
                >,
            >,
        ) -> DispatchResult {
            let poster = ensure_signed(origin)?;
            let new_bounty_id = Self::post_bounty(
                poster.clone(),
                None, // not posting on behalf of org
                description.clone(),
                amount_reserved_for_bounty,
                acceptance_committee,
                supervision_committee,
            )?;
            Self::deposit_event(RawEvent::BountyPosted(new_bounty_id, poster, amount_reserved_for_bounty, description));
            Ok(())
        }
        #[weight = 0]
        fn account_posts_bounty_for_org(
            origin,
            bank_id: OnChainTreasuryID,
            description: T::IpfsReference,
            amount_reserved_for_bounty: BalanceOf<T>,
            acceptance_committee: ResolutionMetadata<
                OrgRep<T::OrgId>,
                T::Signal,
                T::BlockNumber,
            >,
            supervision_committee: Option<
                ResolutionMetadata<
                    OrgRep<T::OrgId>,
                    T::Signal,
                    T::BlockNumber,
                >,
            >,
        ) -> DispatchResult {
            let poster = ensure_signed(origin)?;
            let new_bounty_id = Self::post_bounty(
                poster.clone(),
                Some(bank_id),
                description.clone(),
                amount_reserved_for_bounty,
                acceptance_committee,
                supervision_committee,
            )?;
            Self::deposit_event(RawEvent::BountyPosted(new_bounty_id, poster, amount_reserved_for_bounty, description));
            Ok(())
        }
        #[weight = 0]
        fn account_applies_for_bounty(
            origin,
            bounty_id: T::BountyId,
            description: T::IpfsReference,
            total_amount: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let new_grant_app_id = Self::submit_grant_application(
                submitter.clone(),
                None, // not applying on behalf of org
                bounty_id,
                description,
                total_amount,
            )?;
            Self::deposit_event(RawEvent::BountyApplicationSubmitted(bounty_id, new_grant_app_id, submitter, None, total_amount));
            Ok(())
        }
        #[weight = 0]
        fn account_applies_for_bounty_on_org_behalf(
            origin,
            org_bank: OnChainTreasuryID,
            bounty_id: T::BountyId,
            description: T::IpfsReference,
            total_amount: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let new_grant_app_id = Self::submit_grant_application(
                submitter.clone(),
                Some(org_bank), // not applying on behalf of org
                bounty_id,
                description,
                total_amount,
            )?;
            Self::deposit_event(RawEvent::BountyApplicationSubmitted(bounty_id, new_grant_app_id, submitter, Some(org_bank), total_amount));
            Ok(())
        }
        #[weight = 0]
        fn account_triggers_application_review(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            // TODO: add permissions check, rn naked caller auth
            let app_state = Self::trigger_application_review(
                bounty_id,
                application_id,
            )?;
            Self::deposit_event(RawEvent::ApplicationReviewTriggered(trigger, bounty_id, application_id, app_state));
            Ok(())
        }
        #[weight = 0]
        fn account_sudo_approves_application(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let sudo = ensure_signed(origin)?;
            let app_state = Self::sudo_approve_application(
                sudo.clone(),
                bounty_id,
                application_id,
            )?;
            Self::deposit_event(RawEvent::SudoApprovedBountyApplication(sudo, bounty_id, application_id, app_state));
            Ok(())
        }
        // should be put in on_finalize for prod but this sufficiently demonstrates _callability_
        #[weight = 0]
        fn account_poll_application(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let poller = ensure_signed(origin)?;
            // TODO: add permissions check, rn naked caller auth
            let app_state = Self::poll_application(
                bounty_id,
                application_id,
            )?;
            Self::deposit_event(RawEvent::ApplicationPolled(poller, bounty_id, application_id, app_state));
            Ok(())
        }
        #[weight = 0]
        fn grantee_submits_milestone(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
            submission_reference: T::IpfsReference,
            amount_requested: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let new_milestone_id = Self::submit_milestone(submitter.clone(), bounty_id, application_id, submission_reference.clone(), amount_requested)?;
            Self::deposit_event(RawEvent::MilestoneSubmitted(submitter, bounty_id, application_id, new_milestone_id, amount_requested, submission_reference));
            Ok(())
        }
        #[weight = 0]
        fn account_triggers_milestone_review(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            // TODO: add auth here instead of unpermissioned
            let milestone_status = Self::trigger_milestone_review(bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::MilestoneReviewTriggered(trigger, bounty_id, milestone_id, milestone_status));
            Ok(())
        }
        #[weight = 0]
        fn account_approved_milestone(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let sudo = ensure_signed(origin)?;
            let milestone_status = Self::sudo_approves_milestone(sudo.clone(), bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::SudoApprovedMilestone(sudo, bounty_id, milestone_id, milestone_status));
            Ok(())
        }
        #[weight = 0]
        fn account_polls_milestone(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let poller = ensure_signed(origin)?;
            let milestone_status = Self::poll_milestone(bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::MilestonePolled(poller, bounty_id, milestone_id, milestone_status));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_bounty(id: T::BountyId) -> bool {
        !Self::id_is_available(BIdWrapper::new(id))
    }

    pub fn transfer_milestone_payment(
        sender: BankOrAccount<OnChainTreasuryID, T::AccountId>,
        recipient: BankOrAccount<OnChainTreasuryID, T::AccountId>,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        match (sender, recipient) {
            (
                BankOrAccount::Bank(src_bank_id),
                BankOrAccount::Bank(dest_bank_id),
            ) => {
                let (src_account_id, dest_account_id) = (
                    <bank::Module<T>>::account_id(src_bank_id),
                    <bank::Module<T>>::account_id(dest_bank_id),
                );
                <T as bank::Trait>::Currency::unreserve(
                    &src_account_id,
                    amount,
                );
                <T as bank::Trait>::Currency::transfer(
                    &src_account_id,
                    &dest_account_id,
                    amount,
                    ExistenceRequirement::KeepAlive,
                )
            }
            (
                BankOrAccount::Bank(src_bank_id),
                BankOrAccount::Account(dest_acc),
            ) => {
                let src_account_id = <bank::Module<T>>::account_id(src_bank_id);
                <T as bank::Trait>::Currency::unreserve(
                    &src_account_id,
                    amount,
                );
                <T as bank::Trait>::Currency::transfer(
                    &src_account_id,
                    &dest_acc,
                    amount,
                    ExistenceRequirement::KeepAlive,
                )
            }
            (
                BankOrAccount::Account(sender_acc),
                BankOrAccount::Bank(dest_bank_id),
            ) => {
                <T as bank::Trait>::Currency::unreserve(&sender_acc, amount);
                <T as bank::Trait>::Currency::transfer(
                    &sender_acc,
                    &<bank::Module<T>>::account_id(dest_bank_id),
                    amount,
                    ExistenceRequirement::KeepAlive,
                )
            }
            (
                BankOrAccount::Account(sender_acc),
                BankOrAccount::Account(dest_acc),
            ) => {
                // unreserve and transfer (note reservation associated with posting bounty initially)
                <T as bank::Trait>::Currency::unreserve(&sender_acc, amount);
                <T as bank::Trait>::Currency::transfer(
                    &sender_acc,
                    &dest_acc,
                    amount,
                    ExistenceRequirement::KeepAlive,
                )
            }
        }
    }
}

pub struct BIdWrapper<T> {
    pub id: T,
}

impl<T: Copy> BIdWrapper<T> {
    pub fn new(id: T) -> BIdWrapper<T> {
        BIdWrapper { id }
    }
}

impl<T: Trait> IDIsAvailable<BIdWrapper<T::BountyId>> for Module<T> {
    fn id_is_available(id: BIdWrapper<T::BountyId>) -> bool {
        <LiveBounties<T>>::get(id.id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(T::BountyId, BountyMapID, T::BountyId)>
    for Module<T>
{
    fn id_is_available(id: (T::BountyId, BountyMapID, T::BountyId)) -> bool {
        match id.1 {
            BountyMapID::ApplicationId => {
                <BountyApplications<T>>::get(id.0, id.2).is_none()
            }
            BountyMapID::MilestoneId => {
                <MilestoneSubmissions<T>>::get(id.0, id.2).is_none()
            }
        }
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::BountyId, (T::BountyId, BountyMapID)>
    for Module<T>
{
    fn seeded_generate_unique_id(
        seed: (T::BountyId, BountyMapID),
    ) -> T::BountyId {
        let mut new_id =
            <BountyAssociatedNonces<T>>::get(seed.0, seed.1) + 1u32.into();
        while !Self::id_is_available((seed.0, seed.1, new_id)) {
            new_id += 1u32.into();
        }
        <BountyAssociatedNonces<T>>::insert(seed.0, seed.1, new_id);
        new_id
    }
}

impl<T: Trait> GenerateUniqueID<T::BountyId> for Module<T> {
    fn generate_unique_id() -> T::BountyId {
        let mut id_counter = <BountyNonce<T>>::get() + 1u32.into();
        while !Self::id_is_available(BIdWrapper::new(id_counter)) {
            id_counter += 1u32.into();
        }
        <BountyNonce<T>>::put(id_counter);
        id_counter
    }
}

// this pretty much only exists because if we use direct inheritance for traits with all these generics, it just becomes gross to look at otherwise
impl<T: Trait> ReturnsBountyIdentifier for Module<T> {
    type BountyId = T::BountyId;
}

impl<T: Trait>
    PostBounty<
        T::AccountId,
        T::OrgId,
        OnChainTreasuryID,
        BalanceOf<T>,
        T::IpfsReference,
        ResolutionMetadata<OrgRep<T::OrgId>, T::Signal, T::BlockNumber>,
    > for Module<T>
{
    type BountyInfo = BountyInformation<
        BankOrAccount<OnChainTreasuryID, T::AccountId>,
        T::IpfsReference,
        BalanceOf<T>,
        ResolutionMetadata<OrgRep<T::OrgId>, T::Signal, T::BlockNumber>,
    >;
    fn post_bounty(
        poster: T::AccountId,
        on_behalf_of: Option<OnChainTreasuryID>,
        description: T::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>,
        acceptance_committee: ResolutionMetadata<
            OrgRep<T::OrgId>,
            T::Signal,
            T::BlockNumber,
        >,
        supervision_committee: Option<
            ResolutionMetadata<OrgRep<T::OrgId>, T::Signal, T::BlockNumber>,
        >,
    ) -> Result<Self::BountyId, DispatchError> {
        let bounty_poster: BankOrAccount<OnChainTreasuryID, T::AccountId> =
            if let Some(bank_id) = on_behalf_of {
                <T as bank::Trait>::Currency::reserve(
                    &<bank::Module<T>>::account_id(bank_id),
                    amount_reserved_for_bounty,
                )?;
                BankOrAccount::Bank(bank_id)
            } else {
                <T as bank::Trait>::Currency::reserve(
                    &poster,
                    amount_reserved_for_bounty,
                )?;
                BankOrAccount::Account(poster)
            };
        // form new bounty post
        let new_bounty_post = BountyInformation::new(
            bounty_poster,
            description,
            amount_reserved_for_bounty,
            acceptance_committee,
            supervision_committee,
        );
        // generate unique bounty identifier
        let new_bounty_id = Self::generate_unique_id();
        // insert new bounty
        <LiveBounties<T>>::insert(new_bounty_id, new_bounty_post);
        Ok(new_bounty_id)
    }
}

impl<T: Trait>
    SubmitGrantApplication<
        T::AccountId,
        T::VoteId,
        OnChainTreasuryID,
        BalanceOf<T>,
        T::IpfsReference,
    > for Module<T>
{
    type GrantApp = GrantApplication<
        T::AccountId,
        OnChainTreasuryID,
        BalanceOf<T>,
        T::IpfsReference,
        ApplicationState<T::VoteId>,
    >;
    fn submit_grant_application(
        submitter: T::AccountId,
        bank: Option<OnChainTreasuryID>,
        bounty_id: Self::BountyId,
        description: T::IpfsReference,
        total_amount: BalanceOf<T>,
    ) -> Result<Self::BountyId, DispatchError> {
        // check bounty existence
        let bounty = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::GrantApplicationFailsForBountyThatDNE)?;
        // check that total amount is less than bounty amount
        ensure!(
            bounty.funding_reserved() >= total_amount,
            Error::<T>::GrantApplicationRequestExceedsBountyFundingReserved
        );
        // authorize applications on behalf of org
        if let Some(treasury_id) = bank {
            let the_bank = <bank::Module<T>>::bank_stores(treasury_id).ok_or(
                Error::<T>::CannotApplyForBountyWithOrgBankAccountThatDNE,
            )?;
            // auth is membership check || supervisor
            let authentication = <org::Module<T>>::is_member_of_group(
                the_bank.org(),
                &submitter,
            )
                || <org::Module<T>>::is_organization_supervisor(
                    the_bank.org(),
                    &submitter,
                );
            ensure!(
                authentication,
                Error::<T>::SubmitterNotAuthorizedToSubmitGrantAppForOrg
            );
        }
        // form grant app
        let new_grant_app: GrantApplication<
            T::AccountId,
            OnChainTreasuryID,
            BalanceOf<T>,
            T::IpfsReference,
            ApplicationState<T::VoteId>,
        > = GrantApplication::new(submitter, bank, description, total_amount);
        // generate new grant identifier
        let new_grant_id = Self::seeded_generate_unique_id((
            bounty_id,
            BountyMapID::ApplicationId,
        ));
        // insert new grant application
        <BountyApplications<T>>::insert(bounty_id, new_grant_id, new_grant_app);
        Ok(new_grant_id)
    }
}

impl<T: Trait> SuperviseGrantApplication<T::BountyId, T::AccountId>
    for Module<T>
{
    type AppState = ApplicationState<T::VoteId>;
    fn trigger_application_review(
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfBountyDNE)?;
        let application_to_review =
            <BountyApplications<T>>::get(bounty_id, application_id)
                .ok_or(Error::<T>::CannotReviewApplicationIfApplicationDNE)?;
        // ensure that application is awaiting review (in state from which review can be triggered)
        ensure!(
            application_to_review.state() == ApplicationState::SubmittedAwaitingResponse,
            Error::<T>::ApplicationMustBeSubmittedAwaitingResponseToTriggerReview
        );
        let review_board = bounty_info.acceptance_committee();
        // dispatch vote by acceptance committee
        let new_vote_id = <vote::Module<T>>::open_vote(
            Some(application_to_review.submission()),
            review_board.org(),
            review_board.passage_threshold(),
            review_board.rejection_threshold(),
            review_board.duration(),
        )?;
        // change the application status such that review is started
        let new_application = application_to_review
            .start_review(new_vote_id)
            .ok_or(Error::<T>::ApplicationMustBeSubmittedAwaitingResponseToTriggerReview)?;
        let app_state = new_application.state();
        // insert new application into relevant map
        <BountyApplications<T>>::insert(
            bounty_id,
            application_id,
            new_application,
        );
        Ok(app_state)
    }
    fn sudo_approve_application(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotSudoApproveIfBountyDNE)?;
        // verify that the caller is indeed the sudo
        let authentication = <org::Module<T>>::is_organization_supervisor(
            bounty_info.acceptance_committee().org().org(),
            &caller,
        );
        ensure!(
            authentication,
            Error::<T>::CannotSudoApproveAppIfNotAssignedSudo
        );
        // get the application information
        let app = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotSudoApproveIfGrantAppDNE)?;
        // check that the state of the application satisfies the requirements for approval
        ensure!(
            app.state().awaiting_review(),
            Error::<T>::AppStateCannotBeSudoApprovedForAGrantFromCurrentState
        );
        // approve grant
        let new_application = app.approve_grant();
        let ret_state = new_application.state();
        <BountyApplications<T>>::insert(
            bounty_id,
            application_id,
            new_application,
        );
        Ok(ret_state)
    }
    fn poll_application(
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // check bounty existence for safety
        let _ = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotPollApplicationIfBountyDNE)?;
        // get the application information
        let application_under_review =
            <BountyApplications<T>>::get(bounty_id, application_id)
                .ok_or(Error::<T>::CannotPollApplicationIfApplicationDNE)?;
        match application_under_review.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => {
                // check vote outcome
                let status = <vote::Module<T>>::get_vote_outcome(vote_id)?;
                // match on vote outcome
                match status {
                    VoteOutcome::Approved => {
                        // grant is approved
                        let new_application =
                            application_under_review.approve_grant();
                        // insert into map because application.state() changed => application changed
                        let new_state = new_application.state();
                        <BountyApplications<T>>::insert(
                            bounty_id,
                            application_id,
                            new_application,
                        );
                        Ok(new_state)
                    }
                    VoteOutcome::Rejected => {
                        // remove the application state
                        <BountyApplications<T>>::remove(
                            bounty_id,
                            application_id,
                        );
                        Ok(ApplicationState::Closed)
                    }
                    _ => Ok(application_under_review.state()),
                }
            }
            // nothing changed
            _ => Ok(application_under_review.state()),
        }
    }
}

impl<T: Trait>
    SubmitMilestone<
        T::AccountId,
        T::BountyId,
        T::IpfsReference,
        BalanceOf<T>,
        T::VoteId,
        BankOrAccount<OnChainTreasuryID, T::AccountId>,
    > for Module<T>
{
    type Milestone = MilestoneSubmission<
        T::AccountId,
        T::BountyId,
        T::IpfsReference,
        BalanceOf<T>,
        MilestoneStatus<T::VoteId>,
    >;
    type MilestoneState = MilestoneStatus<T::VoteId>;
    fn submit_milestone(
        submitter: T::AccountId,
        bounty_id: T::BountyId,
        application_id: T::BountyId,
        submission_reference: T::IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<T::BountyId, DispatchError> {
        ensure!(
            Self::is_bounty(bounty_id),
            Error::<T>::CannotSubmitMilestoneIfBaseBountyDNE
        );
        let application =
            <BountyApplications<T>>::get(bounty_id, application_id)
                .ok_or(Error::<T>::CannotSubmitMilestoneIfApplicationDNE)?;
        // ensure that the application has been approved
        ensure!(
            application.state().approved_and_live(),
            Error::<T>::ApplicationMustBeApprovedToSubmitMilestones
        );
        // authenticate submitter in the context of the application
        if let Some(treasury_id) = application.bank() {
            let base_bank = <bank::Module<T>>::bank_stores(treasury_id).ok_or(Error::<T>::InvalidBankReferenceInApplicationThrownInMilestoneSubmission)?;
            let authentication = <org::Module<T>>::is_member_of_group(
                base_bank.org(),
                &submitter,
            )
                || <org::Module<T>>::is_organization_supervisor(
                    base_bank.org(),
                    &submitter,
                );
            ensure!(authentication, Error::<T>::MilestoneSubmissionNotAuthorizedBySubmitterForBankOrgApplication);
        } else {
            let authentication = application.is_submitter(&submitter);
            ensure!(authentication, Error::<T>::MilestoneSubmissionNotAuthorizedBySubmitterForIndividualApplication);
        }
        let new_milestone_submission: Self::Milestone =
            MilestoneSubmission::new(
                submitter,
                application_id,
                submission_reference,
                amount_requested,
            );
        let new_milestone_id = Self::seeded_generate_unique_id((
            bounty_id,
            BountyMapID::MilestoneId,
        ));
        <MilestoneSubmissions<T>>::insert(
            bounty_id,
            new_milestone_id,
            new_milestone_submission,
        );
        Ok(new_milestone_id)
    }
    fn trigger_milestone_review(
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        let bounty = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfBaseBountyDNE)?;
        let review_board = if let Some(board) = bounty.supervision_committee() {
            board
        } else {
            bounty.acceptance_committee()
        };
        let milestone_submission =
            <MilestoneSubmissions<T>>::get(bounty_id, milestone_id).ok_or(
                Error::<T>::CannotTriggerMilestoneReviewIfSubmissionDNE,
            )?;
        // dispatch vote by acceptance committee
        let new_vote_id = <vote::Module<T>>::open_vote(
            Some(milestone_submission.submission()),
            review_board.org(),
            review_board.passage_threshold(),
            review_board.rejection_threshold(),
            review_board.duration(),
        )?;
        // change the application status such that review is started
        let new_milestone_submission = milestone_submission
            .start_review(new_vote_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfSubmissionNotAwaitingResponseAkaWrongState)?;
        let ret_state = new_milestone_submission.state();
        <MilestoneSubmissions<T>>::insert(
            bounty_id,
            milestone_id,
            new_milestone_submission,
        );
        Ok(ret_state)
    }
    fn sudo_approves_milestone(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        let bounty = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotSudoApproveMilestoneIfBaseBountyDNE)?;
        let review_board = if let Some(board) = bounty.supervision_committee() {
            board
        } else {
            bounty.acceptance_committee()
        };
        ensure!(
            <org::Module<T>>::is_organization_supervisor(
                review_board.org().org(),
                &caller
            ),
            Error::<T>::CallerNotAuthorizedToSudoApproveMilestone
        );
        // get the milestone in question
        let milestone_submission =
            <MilestoneSubmissions<T>>::get(bounty_id, milestone_id)
                .ok_or(Error::<T>::CannotSudoApproveMilestoneThatDNE)?;
        // get the relevant application
        let grant_app = <BountyApplications<T>>::get(
            bounty_id,
            milestone_submission.referenced_application(),
        )
        .ok_or(Error::<T>::CannotSudoApproveMilestoneIfBaseAppDNE)?;
        // get grant recipient from grant app
        let grant_recipient: BankOrAccount<OnChainTreasuryID, T::AccountId> =
            if let Some(bank_id) = grant_app.bank() {
                BankOrAccount::Bank(bank_id)
            } else {
                BankOrAccount::Account(grant_app.submitter())
            };
        // execute the transfer and set the relevant state
        let payment_receipt = Self::transfer_milestone_payment(
            bounty.poster(),
            grant_recipient,
            milestone_submission.amount(),
        );
        let new_milestone_submission = if let Ok(()) = payment_receipt {
            milestone_submission
                .set_state(MilestoneStatus::ApprovedAndTransferExecuted)
        } else {
            // will perform the transfer later but it is still approved
            milestone_submission.approve_without_transfer()
        };
        let ret_state = new_milestone_submission.state();
        // insert updated milestone
        <MilestoneSubmissions<T>>::insert(
            bounty_id,
            milestone_id,
            new_milestone_submission,
        );
        Ok(ret_state)
    }
    fn poll_milestone(
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        let bounty = <LiveBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotPollMilestoneSubmissionIfBaseBountyDNE)?;
        // get the milestone in question
        let milestone_submission =
            <MilestoneSubmissions<T>>::get(bounty_id, milestone_id)
                .ok_or(Error::<T>::CannotPollMilestoneThatDNE)?;
        // get the relevant application
        let grant_app = <BountyApplications<T>>::get(
            bounty_id,
            milestone_submission.referenced_application(),
        )
        .ok_or(Error::<T>::CannotPollMilestoneIfBaseAppDNE)?;
        match milestone_submission.state() {
            MilestoneStatus::SubmittedReviewStarted(live_vote_id) => {
                // poll the vote and if approve, make set approved and all that
                let vote_outcome =
                    <vote::Module<T>>::get_vote_outcome(live_vote_id)?;
                if vote_outcome == VoteOutcome::Approved {
                    let poster: BankOrAccount<OnChainTreasuryID, T::AccountId> =
                        bounty.poster();
                    let grant_recipient: BankOrAccount<
                        OnChainTreasuryID,
                        T::AccountId,
                    > = if let Some(bank_id) = grant_app.bank() {
                        BankOrAccount::Bank(bank_id)
                    } else {
                        BankOrAccount::Account(grant_app.submitter())
                    };
                    let payment_receipt = Self::transfer_milestone_payment(
                        poster,
                        grant_recipient,
                        milestone_submission.amount(),
                    );
                    let new_milestone_submission =
                        if let Ok(()) = payment_receipt {
                            milestone_submission.set_state(
                                MilestoneStatus::ApprovedAndTransferExecuted,
                            )
                        } else {
                            milestone_submission.approve_without_transfer()
                        };
                    let ret_state = new_milestone_submission.state();
                    // insert updated milestone
                    <MilestoneSubmissions<T>>::insert(
                        bounty_id,
                        milestone_id,
                        new_milestone_submission,
                    );
                    Ok(ret_state)
                } else {
                    // TODO: change this `if else` to a `match` statement and remove the milestone submission if the vote rejects it? Or need to build a path for updating the milestone and triggering a new vote based on new submission
                    Ok(milestone_submission.state())
                }
            }
            // cannot change anything by polling any other state
            _ => Ok(milestone_submission.state()),
        }
    }
}
