#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The bounty module allows registered organizations with on-chain bank accounts to
//! register as a foundation to post bounties and supervise ongoing grant pursuits.
//!
//! # (Id, Id) Design Justification
//! "WHY so many double_maps in storage with (BountyId, BountyId)?"
//! We use this structure for efficient clean up via double_map.remove_prefix() once
//! a bounty needs to be removed from the storage state so that we can efficiently remove all associated state
//! i.e. applications for a bounty or milestones submitted under a bounty

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
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    bank::{
        BankOrAccount,
        OnChainTreasuryID,
        TransferId,
    },
    bounty::{
        ApplicationState,
        BankSpend,
        BountyInformation,
        BountyMapID,
        GrantApplication,
        MilestoneStatus,
        MilestoneSubmission,
    },
    court::ResolutionMetadata,
    traits::{
        ApproveGrant,
        ApproveWithoutTransfer,
        BountyPermissions,
        GenerateUniqueID,
        GetVoteOutcome,
        GroupMembership,
        IDIsAvailable,
        OpenVote,
        OrganizationSupervisorPermissions,
        PostBounty,
        PostOrgTransfer,
        PostUserTransfer,
        RegisterOrgAccount,
        RegisterOrganization,
        ReturnsBountyIdentifier,
        SeededGenerateUniqueID,
        SetMakeTransfer,
        SpendApprovedGrant,
        StartReview,
        StartTeamConsentPetition,
        SubmitGrantApplication,
        SubmitMilestone,
        SuperviseGrantApplication,
        UseTermsOfAgreement,
    },
    vote::{
        ThresholdConfig,
        VoteOutcome,
    },
};

/// The balances type for this module is inherited from bank
/// - todo, can it match court? is that enforced and can it be
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

// use the court to decide on unachieved milestones...
decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId
    {
        BountyPosted(),
        BountyApplicationSubmitted(),
        // ApprovedBountyApplication(),
        // RejectedBountyApplication(),
        // ApprovedMilestone(),
        PlaceHolder(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PlaceHolderError,
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
                        BankSpend<TransferId<T::BankId>>,
                        T::AccountId
                    >,
                    T::IpfsReference,
                    BalanceOf<T>,
                    ResolutionMetadata<
                        T::OrgId,
                        ThresholdConfig<T::Signal>,
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
                    MilestoneStatus<T::VoteId, BankOrAccount<TransferId<T::BankId>, T::AccountId>>
                >
            >;

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

impl<T: Trait> Module<T> {
    pub fn is_bounty(id: T::BountyId) -> bool {
        !Self::id_is_available(BIdWrapper::new(id))
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
        BankSpend<TransferId<T::BankId>>,
        BalanceOf<T>,
        T::IpfsReference,
        ResolutionMetadata<
            T::OrgId,
            ThresholdConfig<T::Signal>,
            T::BlockNumber,
        >,
    > for Module<T>
{
    type BountyInfo = BountyInformation<
        BankOrAccount<BankSpend<TransferId<T::BankId>>, T::AccountId>,
        T::IpfsReference,
        BalanceOf<T>,
        ResolutionMetadata<
            T::OrgId,
            ThresholdConfig<T::Signal>,
            T::BlockNumber,
        >,
    >;
    fn post_bounty(
        poster: T::AccountId,
        on_behalf_of: Option<BankSpend<TransferId<T::BankId>>>,
        description: T::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>,
        acceptance_committee: ResolutionMetadata<
            T::OrgId,
            ThresholdConfig<T::Signal>,
            T::BlockNumber,
        >,
        supervision_committee: Option<
            ResolutionMetadata<
                T::OrgId,
                ThresholdConfig<T::Signal>,
                T::BlockNumber,
            >,
        >,
    ) -> Result<Self::BountyId, DispatchError> {
        let bounty_poster: BankOrAccount<
            BankSpend<TransferId<T::BankId>>,
            T::AccountId,
        > = if let Some(bank_spend) = on_behalf_of {
            match bank_spend {
                BankSpend::Transfer(full_transfer_id) => {
                    ensure!(
                        <bank::Module<T>>::is_bank(full_transfer_id.id),
                        Error::<T>::CannotPostBountyIfBankReferencedDNE
                    );
                    let transfer_info = <bank::Module<T>>::transfer_info(full_transfer_id.id, full_transfer_id.sub_id).ok_or(Error::<T>::CannotPostBountyOnBehalfOfOrgWithInvalidTransferReference)?;
                    // ensure amount left is above amount_reserved_for_bounty; this check is aggressive because we don't really need to reserve until an application is approved
                    ensure!(transfer_info.amount_left() >= amount_reserved_for_bounty, Error::<T>::CannotPostBountyIfAmountExceedsAmountLeftFromSpendReference);
                    BankOrAccount::Bank(bank_spend)
                }
                BankSpend::Reserved(full_reservation_id) => {
                    ensure!(
                        <bank::Module<T>>::is_bank(full_reservation_id.id),
                        Error::<T>::CannotPostBountyIfBankReferencedDNE
                    );
                    let spend_reservation = <bank::Module<T>>::spend_reservations(full_reservation_id.id, full_reservation_id.sub_id).ok_or(Error::<T>::CannotPostBountyOnBehalfOfOrgWithInvalidSpendReservation)?;
                    // ensure amount left is above amount_reserved_for_bounty; this check is aggressive because we don't really need to reserve until an application is approved
                    ensure!(spend_reservation.amount_left() >= amount_reserved_for_bounty, Error::<T>::CannotPostBountyIfAmountExceedsAmountLeftFromSpendReference);
                    BankOrAccount::Bank(bank_spend)
                }
            }
        } else {
            BankOrAccount::Account(poster.clone())
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
            bounty_info.acceptance_committee().org(),
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
        BankOrAccount<TransferId<T::BankId>, T::AccountId>,
    > for Module<T>
{
    type Milestone = MilestoneSubmission<
        T::AccountId,
        T::BountyId,
        T::IpfsReference,
        BalanceOf<T>,
        MilestoneStatus<
            T::VoteId,
            BankOrAccount<TransferId<T::BankId>, T::AccountId>,
        >,
    >;
    type MilestoneState = MilestoneStatus<
        T::VoteId,
        BankOrAccount<TransferId<T::BankId>, T::AccountId>,
    >;
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
                review_board.org(),
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
        // TODO: extract this logic into an auxiliary function for better readability
        let new_milestone_submission = match (bounty.poster(), grant_recipient)
        {
            // transfer from org bank to org bank using direct forwarding of some existing transfer
            (
                BankOrAccount::Bank(BankSpend::Transfer(full_transfer_id)),
                BankOrAccount::Bank(dest_bank_id),
            ) => {
                let result_transfer_id =
                    <bank::Module<T>>::direct_transfer_to_org(
                        full_transfer_id,
                        dest_bank_id,
                        milestone_submission.amount(),
                    );
                if let Ok(new_transfer_id) = result_transfer_id {
                    milestone_submission.set_make_transfer(BankOrAccount::Bank(new_transfer_id)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
            (
                BankOrAccount::Bank(BankSpend::Reserved(full_reservation_id)),
                BankOrAccount::Bank(_),
            ) => {
                // TODO: check that the dest_bank_id is equal to the reservation in question
                let result_transfer_id =
                    <bank::Module<T>>::transfer_reserved_spend(
                        full_reservation_id,
                        milestone_submission.amount(),
                    );
                if let Ok(new_transfer_id) = result_transfer_id {
                    milestone_submission.set_make_transfer(new_transfer_id).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
            // transfer from org bank to org bank using direct forwarding of some existing transfer
            (
                BankOrAccount::Bank(BankSpend::Transfer(full_transfer_id)),
                BankOrAccount::Account(dest_acc),
            ) => {
                if let Ok(()) = <bank::Module<T>>::direct_transfer_to_account(
                    full_transfer_id,
                    dest_acc.clone(),
                    milestone_submission.amount(),
                ) {
                    milestone_submission.set_make_transfer(BankOrAccount::Account(dest_acc)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
            (
                BankOrAccount::Bank(BankSpend::Reserved(full_reservation_id)),
                BankOrAccount::Account(_),
            ) => {
                // TODO: check that the dest_bank_id is equal to the reservation in question
                let result_transfer_id =
                    <bank::Module<T>>::transfer_reserved_spend(
                        full_reservation_id,
                        milestone_submission.amount(),
                    );
                if let Ok(new_transfer_id) = result_transfer_id {
                    milestone_submission.set_make_transfer(new_transfer_id).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
            (
                BankOrAccount::Account(sender_acc),
                BankOrAccount::Bank(dest_bank_id),
            ) => {
                if let Ok(transfer_id) = <bank::Module<T>>::post_user_transfer(
                    sender_acc,
                    dest_bank_id,
                    milestone_submission.amount(),
                ) {
                    milestone_submission.set_make_transfer(BankOrAccount::Bank(transfer_id)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
            (
                BankOrAccount::Account(sender_acc),
                BankOrAccount::Account(dest_acc),
            ) => {
                // the rest will eventually become as simple as this path with refactoring
                if let Ok(()) = T::Currency::transfer(
                    &sender_acc,
                    &dest_acc,
                    milestone_submission.amount(),
                    ExistenceRequirement::KeepAlive,
                ) {
                    milestone_submission.set_make_transfer(BankOrAccount::Account(dest_acc)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                } else {
                    milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                    // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                }
            }
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
                    // same logic as in sudo_approve, TODO: extract into own method (hard because of associated state)
                    // get grant recipient from grant app
                    let grant_recipient: BankOrAccount<
                        OnChainTreasuryID,
                        T::AccountId,
                    > = if let Some(bank_id) = grant_app.bank() {
                        BankOrAccount::Bank(bank_id)
                    } else {
                        BankOrAccount::Account(grant_app.submitter())
                    };
                    let new_milestone_submission = match (
                        bounty.poster(),
                        grant_recipient,
                    ) {
                        // transfer from org bank to org bank using direct forwarding of some existing transfer
                        (
                            BankOrAccount::Bank(BankSpend::Transfer(
                                full_transfer_id,
                            )),
                            BankOrAccount::Bank(dest_bank_id),
                        ) => {
                            let result_transfer_id =
                                <bank::Module<T>>::direct_transfer_to_org(
                                    full_transfer_id,
                                    dest_bank_id,
                                    milestone_submission.amount(),
                                );
                            if let Ok(new_transfer_id) = result_transfer_id {
                                milestone_submission.set_make_transfer(BankOrAccount::Bank(new_transfer_id)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
                        (
                            BankOrAccount::Bank(BankSpend::Reserved(
                                full_reservation_id,
                            )),
                            BankOrAccount::Bank(_),
                        ) => {
                            // TODO: check that the dest_bank_id is equal to the reservation in question
                            let result_transfer_id =
                                <bank::Module<T>>::transfer_reserved_spend(
                                    full_reservation_id,
                                    milestone_submission.amount(),
                                );
                            if let Ok(new_transfer_id) = result_transfer_id {
                                milestone_submission.set_make_transfer(new_transfer_id).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
                        // transfer from org bank to org bank using direct forwarding of some existing transfer
                        (
                            BankOrAccount::Bank(BankSpend::Transfer(
                                full_transfer_id,
                            )),
                            BankOrAccount::Account(dest_acc),
                        ) => {
                            if let Ok(()) =
                                <bank::Module<T>>::direct_transfer_to_account(
                                    full_transfer_id,
                                    dest_acc.clone(),
                                    milestone_submission.amount(),
                                )
                            {
                                milestone_submission.set_make_transfer(BankOrAccount::Account(dest_acc)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
                        (
                            BankOrAccount::Bank(BankSpend::Reserved(
                                full_reservation_id,
                            )),
                            BankOrAccount::Account(_),
                        ) => {
                            // TODO: check that the dest_bank_id is equal to the reservation in question
                            let result_transfer_id =
                                <bank::Module<T>>::transfer_reserved_spend(
                                    full_reservation_id,
                                    milestone_submission.amount(),
                                );
                            if let Ok(new_transfer_id) = result_transfer_id {
                                milestone_submission.set_make_transfer(new_transfer_id).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            // TODO: consider moving this check ahead and having set_make_transfer return Self instead of Option<Self>
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
                        (
                            BankOrAccount::Account(sender_acc),
                            BankOrAccount::Bank(dest_bank_id),
                        ) => {
                            if let Ok(transfer_id) =
                                <bank::Module<T>>::post_user_transfer(
                                    sender_acc,
                                    dest_bank_id,
                                    milestone_submission.amount(),
                                )
                            {
                                milestone_submission.set_make_transfer(BankOrAccount::Bank(transfer_id)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
                        (
                            BankOrAccount::Account(sender_acc),
                            BankOrAccount::Account(dest_acc),
                        ) => {
                            // the rest will eventually become as simple as this path with refactoring
                            if let Ok(()) = T::Currency::transfer(
                                &sender_acc,
                                &dest_acc,
                                milestone_submission.amount(),
                                ExistenceRequirement::KeepAlive,
                            ) {
                                milestone_submission.set_make_transfer(BankOrAccount::Account(dest_acc)).ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                            } else {
                                milestone_submission.approve_without_transfer().ok_or(Error::<T>::MilestoneCannotBeSudoApprovedFromTheCurrentState)?
                                // TODO: consider moving this check in front and having approve_without_transfer return Self instead of Option<Self>
                            }
                        }
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
