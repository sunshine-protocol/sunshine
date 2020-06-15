#![recursion_limit = "256"]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(non_snake_case)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The bounty module allows registered organizations with on-chain bank accounts to
//! register as a foundation to post bounties and supervise ongoing grant pursuits.

mod tests;

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, Get},
    Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero}, // CheckedAdd, CheckedSub
    DispatchError,
    DispatchResult,
    Permill,
};
use sp_std::{fmt::Debug, prelude::*};

use util::{
    bank::OnChainTreasuryID,
    bounty::{
        ApplicationState, BountyInformation, BountyMapID, GrantApplication, MilestoneStatus,
        MilestoneSubmission, ReviewBoard, TeamID,
    }, //BountyPaymentTracker
    organization::TermsOfAgreement,
    traits::{
        ApproveGrant, ApproveWithoutTransfer, CommitAndTransfer, CreateBounty, GenerateUniqueID,
        GetTeamOrg, GetVoteOutcome, IDIsAvailable, OpenVote, RegisterAccount, RegisterFoundation,
        RegisterOrganization, ReservationMachine, SeededGenerateUniqueID, SetMakeTransfer,
        SpendApprovedGrant, StartReview, StartTeamConsentPetition, SubmitGrantApplication,
        SubmitMilestone, SuperviseGrantApplication, UseTermsOfAgreement,
    }, // UpdateVoteTopic, VoteOnProposal
    vote::{ThresholdConfig, VoteOutcome},
};

/// The balances type for this module
pub type BalanceOf<T> =
    <<T as bank::Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
/// The associate identifier for the bank module
pub type BankAssociatedId<T> = <T as bank::Trait>::BankAssociatedId;

pub trait Trait: frame_system::Trait + org::Trait + vote::Trait + bank::Trait {
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

    /// This is the minimum percent of the total bounty that must be reserved as collateral
    type MinimumBountyCollateralRatio: Get<Permill>;

    /// This is the lower bound for the purported `Balance Available`
    /// => this * MinimumBountyCollateralRatio = MinimumBountyAmount
    type BountyLowerBound: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BountyId,
        Currency = BalanceOf<T>,
        BankAssociatedId = BankAssociatedId<T>,
    {
        FoundationRegisteredFromOnChainBank(OrgId, OnChainTreasuryID),
        FoundationPostedBounty(AccountId, OrgId, BountyId, OnChainTreasuryID, IpfsReference, Currency, Currency),
        // BountyId, Application Id (u32s)
        GrantApplicationSubmittedForBounty(AccountId, BountyId, BountyId, IpfsReference, Currency),
        // BountyId, Application Id (u32s)
        ApplicationReviewTriggered(AccountId, BountyId, BountyId, ApplicationState<TeamID<OrgId, AccountId>, VoteId>),
        SudoApprovedApplication(AccountId, BountyId, BountyId, ApplicationState<TeamID<OrgId, AccountId>, VoteId>),
        ApplicationPolled(BountyId, BountyId, ApplicationState<TeamID<OrgId, AccountId>, VoteId>),
        // BountyId, ApplicationId, MilestoneId (u32s)
        MilestoneSubmitted(AccountId, BountyId, BountyId, BountyId),
        // BountyId, MilestoneId (u32s)
        MilestoneReviewTriggered(AccountId, BountyId, BountyId, MilestoneStatus<OrgId, VoteId, BankAssociatedId>),
        SudoApprovedMilestone(AccountId, BountyId, BountyId, MilestoneStatus<OrgId, VoteId, BankAssociatedId>),
        MilestonePolled(AccountId, BountyId, BountyId, MilestoneStatus<OrgId, VoteId, BankAssociatedId>),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoBankExistsAtInputTreasuryIdForCreatingBounty,
        WithdrawalPermissionsOfBankMustAlignWithCallerToUseForBounty,
        OrganizationBankDoesNotHaveEnoughBalanceToCreateBounty,
        MinimumBountyClaimedAmountMustMeetModuleLowerBound,
        BountyCollateralRatioMustMeetModuleRequirements,
        FoundationMustBeRegisteredToCreateBounty,
        CannotRegisterFoundationFromOrgBankRelationshipThatDNE,
        GrantApplicationFailsIfBountyDNE,
        GrantRequestExceedsAvailableBountyFunds,
        CannotReviewApplicationIfBountyDNE,
        CannotReviewApplicationIfApplicationDNE,
        CannotPollApplicationIfBountyDNE,
        CannotPollApplicationIfApplicationDNE,
        CannotSudoApproveIfBountyDNE,
        CannotSudoApproveAppIfNotAssignedSudo,
        CannotSudoApproveIfGrantAppDNE,
        CannotSubmitMilestoneIfApplicationDNE,
        CannotTriggerMilestoneReviewIfBountyDNE,
        CannotTriggerMilestoneReviewUnlessMember,
        CannotSudoApproveMilestoneIfNotAssignedSudo,
        CannotSudoApproveMilestoneIfMilestoneSubmissionDNE,
        CallerMustBeMemberOfFlatShareGroupToSubmitMilestones,
        CannotTriggerMilestoneReviewIfMilestoneSubmissionDNE,
        CannotPollMilestoneReviewIfBountyDNE,
        CannotPollMilestoneReviewUnlessMember,
        CannotPollMilestoneIfMilestoneSubmissionDNE,
        CannotPollMilestoneIfReferenceApplicationDNE,
        SubmissionIsNotReadyForReview,
        AppStateCannotBeSudoApprovedForAGrantFromCurrentState,
        ApplicationMustBeSubmittedAwaitingResponseToTriggerReview,
        ApplicationMustApprovedAndLiveWithTeamIDMatchingInput,
        MilestoneSubmissionRequestExceedsApprovedApplicationsLimit,
        AccountNotAuthorizedToTriggerApplicationReview,
        ReviewBoardWeightedShapeDoesntSupportPetitionReview,
        ReviewBoardFlatShapeDoesntSupportThresholdReview,
        ApplicationMustBeUnderReviewToPoll,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        /// Uid generation helper for main BountyId
        BountyNonce get(fn bounty_nonce): T::BountyId;

        /// Uid generation helpers for second keys on auxiliary maps
        BountyAssociatedNonces get(fn bounty_associated_nonces): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) BountyMapID => T::BountyId;

        /// Unordered set for tracking foundations as relationships b/t OrgId and OnChainTreasuryID
        pub RegisteredFoundations get(fn registered_foundations): double_map
            hasher(blake2_128_concat) T::OrgId,
            hasher(blake2_128_concat) OnChainTreasuryID => bool;

        /// Posted bounty details
        pub FoundationSponsoredBounties get(fn foundation_sponsored_bounties): map
            hasher(opaque_blake2_256) T::BountyId => Option<
                BountyInformation<
                    T::OrgId,
                    BankAssociatedId<T>,
                    T::IpfsReference,
                    BalanceOf<T>,
                    ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>,
                >
            >;

        /// All bounty applications
        pub BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<GrantApplication<T::AccountId, T::Shares, BalanceOf<T>, T::IpfsReference, ApplicationState<TeamID<T::OrgId, T::AccountId>, T::VoteId>>>;

        /// All milestone submissions
        pub MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<MilestoneSubmission<T::IpfsReference, BalanceOf<T>, T::AccountId, T::BountyId, MilestoneStatus<T::OrgId, T::VoteId, BankAssociatedId<T>>>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn direct__register_foundation_from_existing_bank(
            origin,
            registered_organization: T::OrgId,
            bank_account: OnChainTreasuryID,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // any authorization would need to be HERE
            Self::register_foundation_from_existing_bank(registered_organization, bank_account)?;
            Self::deposit_event(RawEvent::FoundationRegisteredFromOnChainBank(registered_organization, bank_account));
            Ok(())
        }

        #[weight = 0]
        pub fn direct__create_bounty(
            origin,
            registered_organization: T::OrgId,
            description: T::IpfsReference,
            bank_account: OnChainTreasuryID,
            amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
            amount_claimed_available: BalanceOf<T>,  // claimed available amount, not necessarily liquid
            acceptance_committee: ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>,
            supervision_committee: Option<ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>>,
        ) -> DispatchResult {
            let bounty_creator = ensure_signed(origin)?;
            // TODO: AUTH
            let bounty_identifier = Self::create_bounty(
                registered_organization,
                bank_account,
                description.clone(),
                amount_reserved_for_bounty,
                amount_claimed_available,
                acceptance_committee,
                supervision_committee,
            )?;
            Self::deposit_event(RawEvent::FoundationPostedBounty(
                bounty_creator,
                registered_organization,
                bounty_identifier,
                bank_account,
                description,
                amount_reserved_for_bounty,
                amount_claimed_available,
            ));
            Ok(())
        }
        #[weight = 0]
        pub fn direct__submit_grant_application(
            origin,
            bounty_id: T::BountyId,
            description: T::IpfsReference,
            total_amount: BalanceOf<T>,
            terms_of_agreement: TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let new_grant_app_id = Self::submit_grant_application(submitter.clone(), bounty_id, description.clone(), total_amount, terms_of_agreement)?;
            Self::deposit_event(RawEvent::GrantApplicationSubmittedForBounty(submitter, bounty_id, new_grant_app_id, description, total_amount));
            Ok(())
        }
        #[weight = 0]
        pub fn direct__trigger_application_review(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            let application_state = Self::trigger_application_review(bounty_id, application_id)?;
            Self::deposit_event(RawEvent::ApplicationReviewTriggered(trigger, bounty_id, application_id, application_state));
            Ok(())
        }
        #[weight = 0]
        pub fn direct__sudo_approve_application(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let purported_sudo = ensure_signed(origin)?;
            let app_state = Self::sudo_approve_application(purported_sudo.clone(), bounty_id, application_id)?;
            Self::deposit_event(RawEvent::SudoApprovedApplication(purported_sudo, bounty_id, application_id, app_state));
            Ok(())
        }
        #[weight = 0]
        fn any_acc__poll_application(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let app_state = Self::poll_application(bounty_id, application_id)?;
            Self::deposit_event(RawEvent::ApplicationPolled(bounty_id, application_id, app_state));
            Ok(())
        }
        #[weight = 0]
        fn direct__submit_milestone(
            origin,
            bounty_id: T::BountyId,
            application_id: T::BountyId,
            submission_reference: T::IpfsReference,
            amount_requested: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let new_milestone_id = Self::submit_milestone(submitter.clone(), bounty_id, application_id, submission_reference, amount_requested)?;
            Self::deposit_event(RawEvent::MilestoneSubmitted(submitter, bounty_id, application_id, new_milestone_id));
            Ok(())
        }
        #[weight = 0]
        fn direct__trigger_milestone_review(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let trigger = ensure_signed(origin)?;
            let milestone_state = Self::trigger_milestone_review(bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::MilestoneReviewTriggered(trigger, bounty_id, milestone_id, milestone_state));
            Ok(())
        }
        #[weight = 0]
        fn direct__sudo_approves_milestone(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let purported_sudo = ensure_signed(origin)?;
            let milestone_state = Self::sudo_approves_milestone(purported_sudo.clone(), bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::SudoApprovedMilestone(purported_sudo, bounty_id, milestone_id, milestone_state));
            Ok(())
        }
        #[weight = 0]
        fn direct__poll_milestone(
            origin,
            bounty_id: T::BountyId,
            milestone_id: T::BountyId,
        ) -> DispatchResult {
            let poller = ensure_signed(origin)?;
            let milestone_state = Self::poll_milestone(bounty_id, milestone_id)?;
            Self::deposit_event(RawEvent::MilestonePolled(poller, bounty_id, milestone_id, milestone_state));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn collateral_satisfies_module_limits(collateral: BalanceOf<T>, claimed: BalanceOf<T>) -> bool {
        let ratio = Permill::from_rational_approximation(collateral, claimed);
        ratio >= T::MinimumBountyCollateralRatio::get()
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
        <FoundationSponsoredBounties<T>>::get(id.id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(T::BountyId, BountyMapID, T::BountyId)> for Module<T> {
    fn id_is_available(id: (T::BountyId, BountyMapID, T::BountyId)) -> bool {
        match id.1 {
            BountyMapID::ApplicationId => <BountyApplications<T>>::get(id.0, id.2).is_none(),
            BountyMapID::MilestoneId => <MilestoneSubmissions<T>>::get(id.0, id.2).is_none(),
        }
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::BountyId, (T::BountyId, BountyMapID)> for Module<T> {
    fn seeded_generate_unique_id(seed: (T::BountyId, BountyMapID)) -> T::BountyId {
        let mut new_id = <BountyAssociatedNonces<T>>::get(seed.0, seed.1) + 1u32.into();
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

impl<T: Trait> RegisterFoundation<T::OrgId, BalanceOf<T>, T::AccountId> for Module<T> {
    type BankId = OnChainTreasuryID;
    // helper method to quickly bootstrap an organization from a donation
    // -> it should register an on-chain bank account and return the on-chain bank account identifier
    // TODO
    fn register_foundation_from_deposit(
        _from: T::AccountId,
        _for_org: T::OrgId,
        _amount: BalanceOf<T>,
    ) -> Result<Self::BankId, DispatchError> {
        todo!()
    }
    fn register_foundation_from_existing_bank(org: T::OrgId, bank: Self::BankId) -> DispatchResult {
        ensure!(
            <bank::Module<T>>::verify_owner(bank.into(), org),
            Error::<T>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        <RegisteredFoundations<T>>::insert(org, bank, true);
        Ok(())
    }
}

impl<T: Trait>
    CreateBounty<
        T::OrgId,
        BalanceOf<T>,
        T::AccountId,
        T::IpfsReference,
        ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>,
    > for Module<T>
{
    /// Bounty information type
    type BountyInfo = BountyInformation<
        T::OrgId,
        BankAssociatedId<T>,
        T::IpfsReference,
        BalanceOf<T>,
        ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>,
    >;
    /// Bounty identifier type
    type BountyId = T::BountyId;

    /// Helper to screen, prepare and form bounty information object
    fn screen_bounty_creation(
        foundation: T::OrgId, // registered OrgId
        bank_account: Self::BankId,
        description: T::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: ReviewBoard<
            T::OrgId,
            T::AccountId,
            T::IpfsReference,
            ThresholdConfig<T::Signal, Permill>,
        >,
        supervision_committee: Option<
            ReviewBoard<
                T::OrgId,
                T::AccountId,
                T::IpfsReference,
                ThresholdConfig<T::Signal, Permill>,
            >,
        >,
    ) -> Result<Self::BountyInfo, DispatchError> {
        // required registration of relationship between OrgId and OnChainBankId
        ensure!(
            <RegisteredFoundations<T>>::get(foundation, bank_account),
            Error::<T>::FoundationMustBeRegisteredToCreateBounty
        );
        // enforce module constraints for all posted bounties
        ensure!(
            amount_claimed_available >= T::BountyLowerBound::get(),
            Error::<T>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        ensure!(
            Self::collateral_satisfies_module_limits(
                amount_reserved_for_bounty,
                amount_claimed_available,
            ),
            Error::<T>::BountyCollateralRatioMustMeetModuleRequirements
        );

        // reserve `amount_reserved_for_bounty` here by calling into `bank-onchain`
        let spend_reservation_id = <bank::Module<T>>::reserve_for_spend(
            bank_account.into(),
            description.clone(),
            amount_reserved_for_bounty,
            acceptance_committee.org(), // reserved for who? should be the ultimate recipient?
        )?;
        // form the bounty_info
        let new_bounty_info = BountyInformation::new(
            description,
            foundation,
            bank_account,
            spend_reservation_id,
            amount_reserved_for_bounty,
            amount_claimed_available,
            acceptance_committee,
            supervision_committee,
        );
        Ok(new_bounty_info)
    }
    fn create_bounty(
        foundation: T::OrgId, // registered OrgId
        bank_account: Self::BankId,
        description: T::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: ReviewBoard<
            T::OrgId,
            T::AccountId,
            T::IpfsReference,
            ThresholdConfig<T::Signal, Permill>,
        >,
        supervision_committee: Option<
            ReviewBoard<
                T::OrgId,
                T::AccountId,
                T::IpfsReference,
                ThresholdConfig<T::Signal, Permill>,
            >,
        >,
    ) -> Result<T::BountyId, DispatchError> {
        // check that the organization is registered
        ensure!(
            !<org::Module<T>>::id_is_available(foundation),
            Error::<T>::NoBankExistsAtInputTreasuryIdForCreatingBounty
        );
        // creates object and propagates any error
        let new_bounty_info = Self::screen_bounty_creation(
            foundation,
            bank_account,
            description,
            amount_reserved_for_bounty,
            amount_claimed_available,
            acceptance_committee,
            supervision_committee,
        )?;
        // generate unique BountyId for OrgId
        let new_bounty_id = Self::generate_unique_id();
        // insert bounty_info object into storage
        <FoundationSponsoredBounties<T>>::insert(new_bounty_id, new_bounty_info);
        Ok(new_bounty_id)
    }
}

impl<T: Trait>
    SubmitGrantApplication<
        T::OrgId,
        BalanceOf<T>,
        T::AccountId,
        T::IpfsReference,
        ReviewBoard<T::OrgId, T::AccountId, T::IpfsReference, ThresholdConfig<T::Signal, Permill>>,
        TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>,
    > for Module<T>
{
    type GrantApp = GrantApplication<
        T::AccountId,
        T::Shares,
        BalanceOf<T>,
        T::IpfsReference,
        ApplicationState<TeamID<T::OrgId, T::AccountId>, T::VoteId>,
    >;
    fn form_grant_application(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        description: T::IpfsReference,
        total_amount: BalanceOf<T>,
        terms_of_agreement: TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>,
    ) -> Result<Self::GrantApp, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::GrantApplicationFailsIfBountyDNE)?;
        // ensure that the total_amount is below the claimed_available_amount for the referenced bounty
        ensure!(
            bounty_info.claimed_funding_available() >= total_amount, // note this isn't known to be up to date
            Error::<T>::GrantRequestExceedsAvailableBountyFunds
        );
        // form the grant app object and return it
        let grant_app =
            GrantApplication::new(caller, description, total_amount, terms_of_agreement);
        Ok(grant_app)
    }
    fn submit_grant_application(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        description: T::IpfsReference,
        total_amount: BalanceOf<T>,
        terms_of_agreement: TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>,
    ) -> Result<T::BountyId, DispatchError> {
        let formed_grant_app = Self::form_grant_application(
            caller,
            bounty_id,
            description,
            total_amount,
            terms_of_agreement,
        )?;
        let new_application_id =
            Self::seeded_generate_unique_id((bounty_id, BountyMapID::ApplicationId));
        <BountyApplications<T>>::insert(bounty_id, new_application_id, formed_grant_app);
        Ok(new_application_id)
    }
}

impl<T: Trait>
    UseTermsOfAgreement<T::OrgId, TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>>
    for Module<T>
{
    type VoteIdentifier = T::VoteId;
    type TeamIdentifier = TeamID<T::OrgId, T::AccountId>;
    /// This helper method dispatches a vote and returns the information associated with the vote.
    /// - it should only be called from `poll_application`
    fn request_consent_on_terms_of_agreement(
        bounty_org: T::OrgId, // org that supervises the relevant bounty
        terms: TermsOfAgreement<T::AccountId, T::Shares, T::IpfsReference>,
    ) -> Result<(Self::TeamIdentifier, Self::VoteIdentifier), DispatchError> {
        // use terms of agreement to register new org
        let new_team_org = <org::Module<T>>::register_sub_organization(
            bounty_org,
            terms.weighted().into(),
            terms.supervisor(),
            terms.constitution(),
        )?;
        // dispatch consent vote (no end specified for now)
        let consent_vote_id = <vote::Module<T>>::open_unanimous_consent(
            Some(terms.constitution()),
            new_team_org,
            None,
        )?;
        // form the team object
        let new_team = TeamID::new(terms.supervisor(), new_team_org);
        Ok((new_team, consent_vote_id))
    }
}

impl<T: Trait> SuperviseGrantApplication<T::BountyId, T::AccountId> for Module<T> {
    type AppState = ApplicationState<TeamID<T::OrgId, T::AccountId>, T::VoteId>;
    fn trigger_application_review(
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfBountyDNE)?;
        let application_to_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfApplicationDNE)?;
        // ensure that application is awaiting review (in state from which review can be triggered)
        ensure!(
            application_to_review.state() == ApplicationState::SubmittedAwaitingResponse,
            Error::<T>::ApplicationMustBeSubmittedAwaitingResponseToTriggerReview
        );
        let review_board = bounty_info.acceptance_committee();
        // dispatch vote by acceptance committee
        let new_vote_id = <vote::Module<T>>::open_vote(
            review_board.topic(),
            review_board.org(),
            review_board.threshold(),
            None,
            None,
        )?;
        // change the application status such that review is started
        let new_application = application_to_review
            .start_review(new_vote_id)
            .ok_or(Error::<T>::ApplicationMustBeSubmittedAwaitingResponseToTriggerReview)?;
        let app_state = new_application.state();
        // insert new application into relevant map
        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
        Ok(app_state)
    }
    /// Check if the bounty's ReviewBoard has a sudo and if it does, let this person push things through
    /// on behalf of the group but otherwise DO NOT and return an error instead
    /// -> vision is that this person is a SELECTED, TEMPORARY leader
    fn sudo_approve_application(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotSudoApproveIfBountyDNE)?;
        // check that the caller is indeed the sudo
        ensure!(
            bounty_info.acceptance_committee().is_sudo(&caller),
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
        // use terms of agreement to register new org
        let new_team_org = <org::Module<T>>::register_sub_organization(
            bounty_info.foundation(),
            app.terms_of_agreement().weighted().into(),
            app.terms_of_agreement().supervisor(),
            app.terms_of_agreement().constitution(),
        )?;
        // form new team_id
        let new_team_id = TeamID::new(app.terms_of_agreement().supervisor(), new_team_org);
        // store it in the application state
        let new_application = app.approve_grant(new_team_id);
        let ret_state = new_application.state();
        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
        Ok(ret_state)
    }
    /// This returns the AppState but also pushes it along if necessary
    /// - it should be called in on_finalize periodically
    fn poll_application(
        bounty_id: T::BountyId,
        application_id: T::BountyId,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotPollApplicationIfBountyDNE)?;
        // get the application information
        let application_under_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotPollApplicationIfApplicationDNE)?;
        match application_under_review.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => {
                // check vote outcome
                let status = <vote::Module<T>>::get_vote_outcome(vote_id)?;
                // match on vote outcome
                match status {
                    VoteOutcome::ApprovedAndNotExpired => {
                        // passed vote, push state machine along by dispatching triggering team consent
                        let (team_id, team_consent_id) =
                            Self::request_consent_on_terms_of_agreement(
                                bounty_info.foundation(),
                                application_under_review.terms_of_agreement(),
                            )?;
                        let new_application = application_under_review
                            .start_team_consent_petition(team_id, team_consent_id)
                            .ok_or(Error::<T>::CannotPollApplicationIfApplicationDNE)?;
                        // insert into map because application.state() changed => application changed
                        let new_state = new_application.state();
                        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                        Ok(new_state)
                    }
                    VoteOutcome::ApprovedAndExpired => {
                        // passed vote, push state machine along by dispatching triggering team consent
                        let (team_id, team_consent_id) =
                            Self::request_consent_on_terms_of_agreement(
                                bounty_info.foundation(),
                                application_under_review.terms_of_agreement(),
                            )?;
                        let new_application = application_under_review
                            .start_team_consent_petition(team_id, team_consent_id)
                            .ok_or(Error::<T>::CannotPollApplicationIfApplicationDNE)?;
                        // insert into map because application.state() changed => application changed
                        let new_state = new_application.state();
                        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                        Ok(new_state)
                    }
                    _ => Ok(application_under_review.state()),
                }
            }
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(team_id, vote_id) => {
                // check vote outcome
                let status = <vote::Module<T>>::get_vote_outcome(vote_id)?;
                // match on vote outcome
                match status {
                    VoteOutcome::ApprovedAndNotExpired => {
                        let new_application = application_under_review.approve_grant(team_id);
                        let new_state = new_application.state();
                        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                        Ok(new_state)
                    }
                    VoteOutcome::ApprovedAndExpired => {
                        let new_application = application_under_review.approve_grant(team_id);
                        let new_state = new_application.state();
                        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                        Ok(new_state)
                    }
                    _ => {
                        // nothing changed
                        Ok(application_under_review.state())
                    }
                }
            }
            // nothing changed
            _ => Ok(application_under_review.state()),
        }
    }
}

impl<T: Trait>
    SubmitMilestone<
        T::OrgId,
        T::AccountId,
        T::BountyId,
        T::IpfsReference,
        BalanceOf<T>,
        T::VoteId,
        OnChainTreasuryID,
        BankAssociatedId<T>,
    > for Module<T>
{
    type Milestone = MilestoneSubmission<
        T::IpfsReference,
        BalanceOf<T>,
        T::AccountId,
        T::BountyId,
        MilestoneStatus<T::OrgId, T::VoteId, BankAssociatedId<T>>,
    >;
    type MilestoneState = MilestoneStatus<T::OrgId, T::VoteId, BankAssociatedId<T>>;
    fn submit_milestone(
        caller: T::AccountId, // must be from the team, maybe check sudo || flat_org_member
        bounty_id: T::BountyId,
        application_id: T::BountyId,
        submission_reference: T::IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<T::BountyId, DispatchError> {
        // returns Ok(milestone_id)
        // check that the application is in the right state
        let application_to_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotSubmitMilestoneIfApplicationDNE)?;
        // ensure that the amount is less than that approved in the application (NOTE: no change to application here because the funds are not yet formally moved until approved)
        ensure!(
            application_to_review.total_amount() >= amount_requested,
            Error::<T>::MilestoneSubmissionRequestExceedsApprovedApplicationsLimit
        );
        let team_id = application_to_review
            .get_full_team_id()
            .ok_or(Error::<T>::MilestoneSubmissionRequestExceedsApprovedApplicationsLimit)?;
        // form the milestone
        let new_milestone: Self::Milestone = MilestoneSubmission::new(
            team_id.org(),
            caller,
            application_id,
            submission_reference,
            amount_requested,
        );
        // submit the milestone
        let new_milestone_id =
            Self::seeded_generate_unique_id((bounty_id, BountyMapID::MilestoneId));
        <MilestoneSubmissions<T>>::insert(bounty_id, new_milestone_id, new_milestone);
        Ok(new_milestone_id)
    }
    fn trigger_milestone_review(
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the bounty
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfBountyDNE)?;
        // get the milestone submission
        let milestone_submission = <MilestoneSubmissions<T>>::get(bounty_id, milestone_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfMilestoneSubmissionDNE)?;
        // set review board
        let milestone_review_board =
            if let Some(separate_board) = bounty_info.supervision_committee() {
                separate_board
            } else {
                bounty_info.acceptance_committee()
            };
        // check that milestone is in a valid state to trigger a review
        ensure!(
            // TODO: error should tell user that it is already in review when it is instead of returning this error?
            milestone_submission.ready_for_review(),
            Error::<T>::SubmissionIsNotReadyForReview
        );
        // commit reserved spend for transfer before vote begins
        // -> this sets funds aside in case of a positive outcome,
        // it is not _optimistic_, it is fair to add this commitment
        <bank::Module<T>>::commit_reserved_spend_for_transfer(
            bounty_info.bank_account().into(),
            bounty_info.spend_reservation(),
            milestone_submission.amount(),
        )?;
        // dispatch vote among review board on the submission
        let new_vote_id = <vote::Module<T>>::open_vote(
            Some(milestone_submission.submission()),
            milestone_review_board.org(),
            milestone_review_board.threshold(),
            None,
            None,
        )?;
        let new_milestone_submission = milestone_submission
            .start_review(new_vote_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfMilestoneSubmissionDNE)?;
        let milestone_state = new_milestone_submission.state();
        <MilestoneSubmissions<T>>::insert(bounty_id, milestone_id, new_milestone_submission);
        Ok(milestone_state)
    }
    /// Anyone can try to call this but only the sudo can push things through
    fn sudo_approves_milestone(
        caller: T::AccountId,
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the bounty
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotTriggerMilestoneReviewIfBountyDNE)?;
        let milestone_review_board =
            if let Some(separate_board) = bounty_info.supervision_committee() {
                separate_board
            } else {
                bounty_info.acceptance_committee()
            };
        // check if caller is sudo for review board
        ensure!(
            milestone_review_board.is_sudo(&caller),
            Error::<T>::CannotSudoApproveMilestoneIfNotAssignedSudo
        );
        // check that it is in a valid state to approve
        // get the milestone submission
        let milestone_submission = <MilestoneSubmissions<T>>::get(bounty_id, milestone_id)
            .ok_or(Error::<T>::CannotSudoApproveMilestoneIfMilestoneSubmissionDNE)?;
        // check that it is in a valid state to approve
        ensure!(
            milestone_submission.ready_for_review(),
            Error::<T>::SubmissionIsNotReadyForReview
        );
        let team_org_id = milestone_submission
            .get_team_org()
            .ok_or(Error::<T>::SubmissionIsNotReadyForReview)?;

        // commit and transfer control over capital in the same step
        let new_transfer_id = <bank::Module<T>>::commit_and_transfer_spending_power(
            bounty_info.bank_account().into(),
            bounty_info.spend_reservation(),
            milestone_submission.submission(), // reason = hash of milestone submission
            milestone_submission.amount(),
            team_org_id,
        )?;
        let new_milestone_submission = milestone_submission
            .set_make_transfer(bounty_info.bank_account(), new_transfer_id)
            .ok_or(Error::<T>::SubmissionIsNotReadyForReview)?;
        let new_milestone_state = new_milestone_submission.state();
        <MilestoneSubmissions<T>>::insert(bounty_id, milestone_id, new_milestone_submission);
        Ok(new_milestone_state)
    }
    /// Must be called by member of supervision board for
    /// specific milestone (which reserved the bounty) to poll and
    /// push along the milestone (DOES NOT TRIGGER MILESTONE REVIEW)
    fn poll_milestone(
        bounty_id: T::BountyId,
        milestone_id: T::BountyId,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the bounty
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotPollMilestoneReviewIfBountyDNE)?;
        // get the milestone submission
        let milestone_submission = <MilestoneSubmissions<T>>::get(bounty_id, milestone_id)
            .ok_or(Error::<T>::CannotPollMilestoneIfMilestoneSubmissionDNE)?;
        // poll the state of the submission and return the result
        // -> pushes along if milestone review passes
        match milestone_submission.state() {
            MilestoneStatus::SubmittedReviewStarted(org_id, vote_id) => {
                // poll the vote_id
                let passed = <vote::Module<T>>::get_vote_outcome(vote_id)?;
                // match on vote outcome
                let latest_submission: Self::Milestone = match passed {
                    VoteOutcome::ApprovedAndNotExpired => {
                        let application = <BountyApplications<T>>::get(
                            bounty_id,
                            milestone_submission.application_id(),
                        )
                        .ok_or(Error::<T>::CannotPollMilestoneIfReferenceApplicationDNE)?;
                        let new_milestone_submission = if let Some(new_application) =
                            application.spend_approved_grant(milestone_submission.amount())
                        {
                            // make the transfer
                            let transfer_id = <bank::Module<T>>::transfer_spending_power(
                                bounty_info.bank_account().into(),
                                milestone_submission.submission(), // reason = hash of milestone submission
                                bounty_info.spend_reservation(),
                                milestone_submission.amount(),
                                org_id, // uses the weighted share issuance by default to enforce payout structure
                            )?;
                            // insert updated application into storage
                            <BountyApplications<T>>::insert(
                                bounty_id,
                                milestone_submission.application_id(),
                                new_application,
                            );
                            milestone_submission
                                .set_make_transfer(bounty_info.bank_account(), transfer_id)
                                .ok_or(Error::<T>::CannotPollMilestoneIfMilestoneSubmissionDNE)
                        } else {
                            // can't afford to the make the transfer at the moment
                            milestone_submission
                                .approve_without_transfer()
                                .ok_or(Error::<T>::CannotPollMilestoneIfMilestoneSubmissionDNE)
                        }?;
                        <MilestoneSubmissions<T>>::insert(
                            bounty_id,
                            milestone_id,
                            new_milestone_submission.clone(),
                        );
                        new_milestone_submission
                    }
                    VoteOutcome::ApprovedAndExpired => {
                        let application = <BountyApplications<T>>::get(
                            bounty_id,
                            milestone_submission.application_id(),
                        )
                        .ok_or(Error::<T>::CannotPollMilestoneIfReferenceApplicationDNE)?;
                        let new_milestone_submission = if let Some(new_application) =
                            application.spend_approved_grant(milestone_submission.amount())
                        {
                            // make the transfer
                            let transfer_id = <bank::Module<T>>::transfer_spending_power(
                                bounty_info.bank_account().into(),
                                milestone_submission.submission(), // reason = hash of milestone submission
                                bounty_info.spend_reservation(),
                                milestone_submission.amount(),
                                org_id, // uses the weighted share issuance by default to enforce payout structure
                            )?;
                            // insert updated application into storage
                            <BountyApplications<T>>::insert(
                                bounty_id,
                                milestone_submission.application_id(),
                                new_application,
                            );
                            milestone_submission
                                .set_make_transfer(bounty_info.bank_account(), transfer_id)
                                .ok_or(Error::<T>::CannotPollMilestoneIfMilestoneSubmissionDNE)
                        } else {
                            // can't afford to the make the transfer at the moment
                            milestone_submission
                                .approve_without_transfer()
                                .ok_or(Error::<T>::CannotPollMilestoneIfMilestoneSubmissionDNE)
                        }?;
                        <MilestoneSubmissions<T>>::insert(
                            bounty_id,
                            milestone_id,
                            new_milestone_submission.clone(),
                        );
                        new_milestone_submission
                    }
                    _ => milestone_submission,
                };
                Ok(latest_submission.state())
            }
            MilestoneStatus::ApprovedButNotTransferred(org_id) => {
                // try to make the transfer again and change the state
                let application =
                    <BountyApplications<T>>::get(bounty_id, milestone_submission.application_id())
                        .ok_or(Error::<T>::CannotPollMilestoneIfReferenceApplicationDNE)?;
                if let Some(new_application) =
                    application.spend_approved_grant(milestone_submission.amount())
                {
                    // make the transfer
                    let transfer_id = <bank::Module<T>>::transfer_spending_power(
                        bounty_info.bank_account().into(),
                        milestone_submission.submission(), // reason = hash of milestone submission
                        bounty_info.spend_reservation(),
                        milestone_submission.amount(),
                        org_id,
                    )?;
                    let new_milestone_submission = milestone_submission
                        .set_make_transfer(bounty_info.bank_account(), transfer_id)
                        .ok_or(Error::<T>::CannotPollMilestoneIfReferenceApplicationDNE)?;
                    let new_milestone_state = new_milestone_submission.state();
                    <MilestoneSubmissions<T>>::insert(
                        bounty_id,
                        milestone_id,
                        new_milestone_submission,
                    );
                    <BountyApplications<T>>::insert(
                        bounty_id,
                        milestone_submission.application_id(),
                        new_application,
                    );
                    Ok(new_milestone_state)
                } else {
                    Ok(milestone_submission.state())
                }
            }
            _ => Ok(milestone_submission.state()),
        }
    }
}
