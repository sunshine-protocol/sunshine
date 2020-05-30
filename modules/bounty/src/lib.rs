#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::{Currency, Get},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult, Permill};
use sp_std::prelude::*;

use util::{
    bank::{BankMapID, OnChainTreasuryID, WithdrawalPermissions},
    bounty::{
        ApplicationState, BountyInformation, BountyMapID, GrantApplication, MilestoneSubmission,
        ReviewBoard, TeamID, VoteID,
    }, //BountyPaymentTracker
    organization::{ShareID, TermsOfAgreement},
    traits::{
        ApplyVote, ApproveGrant, Approved, BankDepositsAndSpends, BankReservations, BankSpends,
        BankStorageInfo, CheckBankBalances, CheckVoteStatus, CreateBounty, DepositIntoBank,
        FoundationParts, GenerateUniqueID, GenerateUniqueKeyID, GetInnerOuterShareGroups,
        GetVoteOutcome, IDIsAvailable, MintableSignal, OnChainBank, OpenPetition,
        OpenShareGroupVote, OrgChecks, OrganizationDNS, OwnershipProportionCalculations,
        RegisterBankAccount, RegisterFoundation, RegisterShareGroup, SeededGenerateUniqueID,
        ShareGroupChecks, SignPetition, StartApplicationReviewPetition, StartTeamConsentPetition,
        SubmitGrantApplication, SubmitMilestone, SuperviseGrantApplication, SupervisorPermissions,
        TermSheetExit, ThresholdVote, UpdatePetition, UseTermsOfAgreement, VoteOnProposal,
        WeightedShareIssuanceWrapper, WeightedShareWrapper,
    }, //RequestChanges
    voteyesno::{SupportedVoteTypes, ThresholdConfig},
};

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The organization identfier
pub type OrgId = u32;
/// The bounty identifier
pub type BountyId = u32;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::Organization as WeightedShareWrapper<
    u32,
    u32,
    <T as frame_system::Trait>::AccountId,
>>::Shares;
/// The balances type for this module
pub type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
/// The signal type for this module
pub type SignalOf<T> = <<T as Trait>::VoteYesNo as ThresholdVote>::Signal;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    /// This type wraps `membership`, `shares-membership`, and `shares-atomic`
    /// - it MUST be the same instance inherited by the bank module associated type
    type Organization: OrgChecks<u32, Self::AccountId>
        + ShareGroupChecks<u32, ShareID, Self::AccountId>
        + GetInnerOuterShareGroups<u32, ShareID, Self::AccountId>
        + SupervisorPermissions<u32, ShareID, Self::AccountId>
        + WeightedShareWrapper<u32, u32, Self::AccountId>
        + WeightedShareIssuanceWrapper<u32, u32, Self::AccountId, Permill>
        + RegisterShareGroup<u32, ShareID, Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<u32, Self::AccountId, IpfsReference>;

    // TODO: start with spending functionality with balances for milestones
    // - then extend to offchain bank interaction (try to mirror logic/calls)
    type Bank: IDIsAvailable<OnChainTreasuryID>
        + IDIsAvailable<(OnChainTreasuryID, BankMapID)>
        + GenerateUniqueKeyID<(OnChainTreasuryID, BankMapID)>
        + GenerateUniqueID<OnChainTreasuryID>
        + OnChainBank
        + RegisterBankAccount<
            Self::AccountId,
            WithdrawalPermissions<Self::AccountId>,
            BalanceOf<Self>,
        > + OwnershipProportionCalculations<
            Self::AccountId,
            WithdrawalPermissions<Self::AccountId>,
            BalanceOf<Self>,
            Permill,
        > + BankDepositsAndSpends<BalanceOf<Self>>
        + CheckBankBalances<BalanceOf<Self>>
        + DepositIntoBank<
            Self::AccountId,
            WithdrawalPermissions<Self::AccountId>,
            IpfsReference,
            BalanceOf<Self>,
        > + BankReservations<
            Self::AccountId,
            WithdrawalPermissions<Self::AccountId>,
            BalanceOf<Self>,
            IpfsReference,
        > + BankSpends<Self::AccountId, WithdrawalPermissions<Self::AccountId>, BalanceOf<Self>>
        + BankStorageInfo<Self::AccountId, WithdrawalPermissions<Self::AccountId>, BalanceOf<Self>>
        + TermSheetExit<Self::AccountId, BalanceOf<Self>>;

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VotePetition: IDIsAvailable<u32>
        + GenerateUniqueID<u32>
        + GetVoteOutcome
        + OpenPetition<IpfsReference, Self::BlockNumber>
        + SignPetition<Self::AccountId, IpfsReference>
        + UpdatePetition<Self::AccountId, IpfsReference>; // + RequestChanges<Self::AccountId, IpfsReference>

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VoteYesNo: IDIsAvailable<u32>
        + GenerateUniqueID<u32>
        + MintableSignal<Self::AccountId, Self::BlockNumber, Permill>
        + GetVoteOutcome
        + ThresholdVote
        + OpenShareGroupVote<Self::AccountId, Self::BlockNumber, Permill>
        + ApplyVote
        + CheckVoteStatus
        + VoteOnProposal<Self::AccountId, IpfsReference, Self::BlockNumber, Permill>;

    // every bounty must have a bank account set up with this minimum amount of collateral
    // _idea_: allow use of offchain bank s.t. both sides agree on how much one side demonstrated ownership of to the other
    // --> eventually, we might use proofs of ownership on other chains (like however lockdrop worked)
    type MinimumBountyCollateralRatio: Get<Permill>;

    // combined with the above constant, this defines constraints on bounties posted
    type BountyLowerBound: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Currency = BalanceOf<T>,
    {
        FoundationPostedBounty(AccountId, OrgId, BountyId, OnChainTreasuryID, IpfsReference, Currency, Currency),
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
        AppStateCannotBeSudoApprovedForAGrantFromCurrentState,
        ApplicationMustBeSubmittedAwaitingResponseToTriggerReview,
        AccountNotAuthorizedToTriggerApplicationReview,
        ReviewBoardWeightedShapeDoesntSupportPetitionReview,
        ReviewBoardFlatShapeDoesntSupportThresholdReview,
        ApplicationMustBeUnderReviewToPoll,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        BountyNonce get(fn bounty_nonce): BountyId;

        BountyAssociatedNonces get(fn bounty_associated_nonces): double_map
            hasher(opaque_blake2_256) BountyId,
            hasher(opaque_blake2_256) BountyMapID => u32;

        // unordered set for tracking foundations as relationships b/t OrgId and OnChainTreasuryID
        RegisteredFoundations get(fn registered_foundations): double_map
            hasher(blake2_128_concat) OrgId,
            hasher(blake2_128_concat) OnChainTreasuryID => bool;

        // Teams that are pursuing live grants
        RegisteredTeams get(fn registered_teams): map
            hasher(blake2_128_concat) TeamID<T::AccountId> => bool;

        // TODO: helper method for getting all the bounties for an organization
        FoundationSponsoredBounties get(fn foundation_sponsored_bounties): map
            hasher(opaque_blake2_256) BountyId => Option<
                BountyInformation<T::AccountId, IpfsReference, ThresholdConfig<SignalOf<T>, Permill>, BalanceOf<T>>
            >;

        // second key is an ApplicationId
        BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) BountyId,
            hasher(opaque_blake2_256) u32 => Option<GrantApplication<T::AccountId, SharesOf<T>, BalanceOf<T>, IpfsReference>>;

        // second key is a MilestoneId
        MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) BountyId,
            hasher(opaque_blake2_256) u32 => Option<MilestoneSubmission<IpfsReference, BalanceOf<T>, T::AccountId>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_foundation_from_existing_on_chain_bank(
            origin,
            registered_organization: OrgId,
            bank_account: OnChainTreasuryID,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // any authorization would need to be HERE
            Self::register_foundation_from_existing_bank(registered_organization, bank_account)?;
            // TODO: deposit relevant event
            Ok(())
        }

        #[weight = 0]
        fn create_bounty_on_behalf_of_foundation(
            origin,
            registered_organization: OrgId,
            description: IpfsReference,
            bank_account: OnChainTreasuryID,
            amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
            amount_claimed_available: BalanceOf<T>,  // claimed available amount, not necessarily liquid
            acceptance_committee: ReviewBoard<T::AccountId, IpfsReference, ThresholdConfig<SignalOf<T>, Permill>>,
            supervision_committee: Option<ReviewBoard<T::AccountId, IpfsReference, ThresholdConfig<SignalOf<T>, Permill>>>,
        ) -> DispatchResult {
            let bounty_creator = ensure_signed(origin)?;
            // TODO: need to verify bank_account ownership by registered_organization somehow
            // -> may just need to add this check to the spend reservation implicitly
            let bounty_identifier = Self::create_bounty(
                registered_organization,
                bounty_creator.clone(),
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
    }
}

impl<T: Trait> Module<T> {
    fn collateral_satisfies_module_limits(collateral: BalanceOf<T>, claimed: BalanceOf<T>) -> bool {
        let ratio = Permill::from_rational_approximation(collateral, claimed);
        ratio >= T::MinimumBountyCollateralRatio::get()
    }
    // In the future, consider this as a method in a trait for inputting
    // application and returning dispatched VoteId based on context
    // (which is what the method that calls this is doing...)
    fn account_can_trigger_application_review(
        account: &T::AccountId,
        acceptance_committee: ReviewBoard<
            T::AccountId,
            IpfsReference,
            ThresholdConfig<SignalOf<T>, Permill>,
        >,
    ) -> bool {
        match acceptance_committee {
            ReviewBoard::FlatPetitionReview(_, org_id, share_id, _, _, _) => {
                <<T as Trait>::Organization as ShareGroupChecks<
                    u32,
                    ShareID,
                    T::AccountId,
                >>::check_membership_in_share_group(org_id, ShareID::Flat(share_id).into(), account)
            },
            ReviewBoard::WeightedThresholdReview(_, org_id, share_id, _, _) => {
                <<T as Trait>::Organization as ShareGroupChecks<
                    u32,
                    ShareID,
                    T::AccountId,
                >>::check_membership_in_share_group(org_id, ShareID::WeightedAtomic(share_id).into(), account)
            },
        }
    }
    fn check_vote_status(vote_id: VoteID) -> Result<bool, DispatchError> {
        match vote_id {
            VoteID::Petition(petition_id) => {
                let outcome = <<T as Trait>::VotePetition as GetVoteOutcome>::get_vote_outcome(
                    petition_id.into(),
                )?;
                Ok(outcome.approved())
            }
            VoteID::Threshold(threshold_id) => {
                let outcome = <<T as Trait>::VoteYesNo as GetVoteOutcome>::get_vote_outcome(
                    threshold_id.into(),
                )?;
                Ok(outcome.approved())
            }
        }
    }
    fn dispatch_threshold_review(
        organization: u32,
        weighted_share_id: u32,
        vote_type: SupportedVoteTypes,
        threshold: ThresholdConfig<SignalOf<T>, Permill>,
        duration: Option<T::BlockNumber>,
    ) -> Result<VoteID, DispatchError> {
        let id: u32 = <<T as Trait>::VoteYesNo as OpenShareGroupVote<
            T::AccountId,
            T::BlockNumber,
            Permill,
        >>::open_share_group_vote(
            organization,
            weighted_share_id,
            vote_type.into(),
            threshold.into(),
            duration,
        )?
        .into();
        Ok(VoteID::Threshold(id))
    }
    fn dispatch_unanimous_petition_review(
        organization: u32,
        flat_share_id: u32,
        topic: Option<IpfsReference>,
        duration: Option<T::BlockNumber>,
    ) -> Result<VoteID, DispatchError> {
        let id: u32 = <<T as Trait>::VotePetition as OpenPetition<
            IpfsReference,
            T::BlockNumber,
        >>::open_unanimous_approval_petition(
            organization, flat_share_id, topic, duration
        )?
        .into();
        Ok(VoteID::Petition(id))
    }
    fn dispatch_petition_review(
        organization: u32,
        flat_share_id: u32,
        topic: Option<IpfsReference>,
        required_support: u32,
        required_against: Option<u32>,
        duration: Option<T::BlockNumber>,
    ) -> Result<VoteID, DispatchError> {
        let id: u32 = <<T as Trait>::VotePetition as OpenPetition<
            IpfsReference,
            T::BlockNumber,
        >>::open_petition(
            organization,
            flat_share_id,
            topic,
            required_support,
            required_against,
            duration,
        )?
        .into();
        Ok(VoteID::Petition(id))
    }
}

impl<T: Trait> IDIsAvailable<BountyId> for Module<T> {
    fn id_is_available(id: BountyId) -> bool {
        <FoundationSponsoredBounties<T>>::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(BountyId, BountyMapID, u32)> for Module<T> {
    fn id_is_available(id: (BountyId, BountyMapID, u32)) -> bool {
        match id.1 {
            BountyMapID::ApplicationId => <BountyApplications<T>>::get(id.0, id.2).is_none(),
            BountyMapID::MilestoneId => <MilestoneSubmissions<T>>::get(id.0, id.2).is_none(),
        }
    }
}

impl<T: Trait> SeededGenerateUniqueID<u32, (BountyId, BountyMapID)> for Module<T> {
    fn seeded_generate_unique_id(seed: (BountyId, BountyMapID)) -> u32 {
        let mut new_id = <BountyAssociatedNonces>::get(seed.0, seed.1) + 1u32;
        match seed.1 {
            BountyMapID::ApplicationId => {
                while !Self::id_is_available((seed.0, seed.1, new_id)) {
                    new_id += 1u32;
                }
                <BountyAssociatedNonces>::insert(seed.0, seed.1, new_id);
                new_id
            }
            BountyMapID::MilestoneId => {
                while !Self::id_is_available((seed.0, seed.1, new_id)) {
                    new_id += 1u32;
                }
                <BountyAssociatedNonces>::insert(seed.0, seed.1, new_id);
                new_id
            }
        }
    }
}

impl<T: Trait> GenerateUniqueID<BountyId> for Module<T> {
    fn generate_unique_id() -> BountyId {
        let mut id_counter = BountyNonce::get() + 1;
        while !Self::id_is_available(id_counter) {
            id_counter += 1;
        }
        BountyNonce::put(id_counter);
        id_counter
    }
}

impl<T: Trait> FoundationParts for Module<T> {
    type OrgId = OrgId;
    type BountyId = BountyId;
    type BankId = OnChainTreasuryID;
    type TeamId = TeamID<T::AccountId>;
    type MultiShareId = ShareID;
    type MultiVoteId = VoteID;
}

impl<T: Trait> RegisterFoundation<BalanceOf<T>, T::AccountId> for Module<T> {
    // helper method to quickly bootstrap an organization form a donation
    // -> it should register an on-chain bank account and return the on-chain bank account identifier
    // TODO
    fn register_foundation_from_donation_deposit(
        _from: T::AccountId,
        _for_org: Self::OrgId,
        _amount: BalanceOf<T>,
    ) -> Result<Self::BankId, DispatchError> {
        todo!()
    }
    fn register_foundation_from_existing_bank(
        org: Self::OrgId,
        bank: Self::BankId,
    ) -> DispatchResult {
        ensure!(
            <<T as Trait>::Bank as RegisterBankAccount<
                T::AccountId,
                WithdrawalPermissions<T::AccountId>,
                BalanceOf<T>,
            >>::check_bank_owner(bank.into(), org.into()),
            Error::<T>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        RegisteredFoundations::insert(org, bank, true);
        Ok(())
    }
}

impl<T: Trait> CreateBounty<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type BountyInfo = BountyInformation<
        T::AccountId,
        IpfsReference,
        ThresholdConfig<SignalOf<T>, Permill>,
        BalanceOf<T>,
    >;
    // smpl vote config for this module in particular
    type ReviewCommittee =
        ReviewBoard<T::AccountId, IpfsReference, ThresholdConfig<SignalOf<T>, Permill>>;
    // helper to screen, prepare and form bounty information object
    fn screen_bounty_creation(
        foundation: u32, // registered OrgId
        caller: T::AccountId,
        bank_account: Self::BankId,
        description: IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<Self::BountyInfo, DispatchError> {
        // required registration of relationship between OrgId and OnChainBankId
        ensure!(
            RegisteredFoundations::get(foundation, bank_account),
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
        let spend_reservation_id = <<T as Trait>::Bank as BankReservations<
            T::AccountId,
            WithdrawalPermissions<T::AccountId>,
            BalanceOf<T>,
            IpfsReference,
        >>::reserve_for_spend(
            caller,
            bank_account.into(),
            description.clone(),
            amount_reserved_for_bounty,
            acceptance_committee.clone().into(),
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
        foundation: u32, // registered OrgId
        caller: T::AccountId,
        bank_account: Self::BankId,
        description: IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<u32, DispatchError> {
        // quick lint, check that the organization is already registered in the org module
        ensure!(
            <<T as Trait>::Organization as OrgChecks<u32, <T as frame_system::Trait>::AccountId>>::check_org_existence(foundation),
            Error::<T>::NoBankExistsAtInputTreasuryIdForCreatingBounty
        );
        // creates object and propagates any error related to invalid creation inputs
        let new_bounty_info = Self::screen_bounty_creation(
            foundation,
            caller,
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

impl<T: Trait> SubmitGrantApplication<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type GrantApp = GrantApplication<T::AccountId, SharesOf<T>, BalanceOf<T>, IpfsReference>;
    fn form_grant_application(
        bounty_id: u32,
        description: IpfsReference,
        total_amount: BalanceOf<T>,
        terms_of_agreement: Self::TermsOfAgreement,
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
        let grant_app = GrantApplication::new(description, total_amount, terms_of_agreement);
        Ok(grant_app)
    }
    fn submit_grant_application(
        bounty_id: u32,
        description: IpfsReference,
        total_amount: BalanceOf<T>,
        terms_of_agreement: Self::TermsOfAgreement,
    ) -> Result<u32, DispatchError> {
        let formed_grant_app =
            Self::form_grant_application(bounty_id, description, total_amount, terms_of_agreement)?;
        let new_application_id =
            Self::seeded_generate_unique_id((bounty_id, BountyMapID::ApplicationId));
        <BountyApplications<T>>::insert(bounty_id, new_application_id, formed_grant_app);
        Ok(new_application_id)
    }
}

impl<T: Trait> UseTermsOfAgreement<T::AccountId> for Module<T> {
    type TermsOfAgreement = TermsOfAgreement<T::AccountId, SharesOf<T>>;
    // should only be called from `poll_application`
    fn request_consent_on_terms_of_agreement(
        bounty_org: u32, // org that supervises the relevant bounty
        terms: TermsOfAgreement<T::AccountId, SharesOf<T>>,
    ) -> Result<(ShareID, VoteID), DispatchError> {
        // register an appropriate flat share identity for this team as an outer share group in the org
        let outer_flat_share_id_for_team =
            <<T as Trait>::Organization as RegisterShareGroup<
                u32,
                ShareID,
                T::AccountId,
                SharesOf<T>,
            >>::register_outer_flat_share_group(bounty_org, terms.flat())?;
        let rewrapped_share_id: ShareID = outer_flat_share_id_for_team.into();
        let unwrapped_share_id: u32 = outer_flat_share_id_for_team.into();
        // dispatch vote on consent on the terms for the bounty, requires full consent
        let vote_id =
            Self::dispatch_unanimous_petition_review(bounty_org, unwrapped_share_id, None, None)?;
        Ok((rewrapped_share_id, vote_id))
    }
    fn approve_grant_to_register_team(
        bounty_org: u32,
        flat_share_id: u32,
        terms: Self::TermsOfAgreement,
    ) -> Result<Self::TeamId, DispatchError> {
        // use the terms to register the team
        let weighted_share_id =
            <<T as Trait>::Organization as RegisterShareGroup<
                u32,
                ShareID,
                T::AccountId,
                SharesOf<T>,
            >>::register_outer_weighted_share_group(bounty_org, terms.weighted())?;
        // TODO: enum with `TermsOfAgreement` OR the registered suborg identifier
        // and check if there is already a registered team and put that TeamID here instead
        // create the new team object
        let new_team = TeamID::new(
            bounty_org,
            terms.supervisor(),
            flat_share_id,
            weighted_share_id.into(),
        );
        // insert the new team object into storage
        <RegisteredTeams<T>>::insert(new_team.clone(), true);
        Ok(new_team)
    }
}

impl<T: Trait> SuperviseGrantApplication<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type AppState = ApplicationState<T::AccountId>;
    fn trigger_application_review(
        trigger: T::AccountId, // must be authorized to trigger in context of objects
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfBountyDNE)?;
        // get the application that is under review
        let application_to_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfApplicationDNE)?;
        // change the bounty application state to UnderReview
        ensure!(
            application_to_review.state() == ApplicationState::SubmittedAwaitingResponse,
            Error::<T>::ApplicationMustBeSubmittedAwaitingResponseToTriggerReview
        );
        // check if the trigger is authorized to trigger a vote on this application
        // --- for now, this will consist of a membership check for the bounty_info.acceptance_committee
        ensure!(
            Self::account_can_trigger_application_review(
                &trigger,
                bounty_info.acceptance_committee()
            ),
            Error::<T>::AccountNotAuthorizedToTriggerApplicationReview
        );
        // vote should dispatch based on the acceptance_committee variant here
        let new_vote_id = match bounty_info.acceptance_committee() {
            ReviewBoard::FlatPetitionReview(
                _,
                org_id,
                flat_share_id,
                required_support,
                required_against,
                _,
            ) => {
                // TODO: create two defaults (1) global for unset, here
                // (2) for each foundation, enable setting a default for this?
                Self::dispatch_petition_review(
                    org_id,
                    flat_share_id,
                    None,
                    required_support,
                    required_against,
                    None,
                )?
            }
            // TODO: add thresholds to this ReviewBoard variant instead of relying on passed defaults
            // - or we could get the thresholds from some config stored somewhere and gotten
            // in the called helper methods (`dispatch_threshold_review`)
            ReviewBoard::WeightedThresholdReview(
                _,
                org_id,
                weighted_share_id,
                vote_type,
                threshold,
            ) => Self::dispatch_threshold_review(
                org_id,
                weighted_share_id,
                vote_type,
                threshold,
                None,
            )?,
        };
        // change the application status such that review is started
        let new_application = application_to_review.start_application_review_petition(new_vote_id);
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
        bounty_id: u32,
        application_id: u32,
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
            app.state().live(),
            Error::<T>::AppStateCannotBeSudoApprovedForAGrantFromCurrentState
        );
        // sudo approve vote, push state machine along by dispatching team consent
        let (team_flat_share_id, team_consent_vote_id) =
            Self::request_consent_on_terms_of_agreement(
                bounty_info.foundation(),
                app.terms_of_agreement(),
            )?;
        let new_application =
            app.start_team_consent_petition(team_flat_share_id, team_consent_vote_id);
        let ret_state = new_application.state();
        <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
        Ok(ret_state)
    }
    /// This returns the AppState but also pushes it along if necessary
    /// - it should be called in on_finalize periodically
    fn poll_application(
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotPollApplicationIfBountyDNE)?;
        // get the application information
        let application_under_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotPollApplicationIfApplicationDNE)?;
        match application_under_review.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(wrapped_vote_id) => {
                // check the vote status
                let status = Self::check_vote_status(wrapped_vote_id)?;
                if status {
                    // passed vote, push state machine along by dispatching triggering team consent
                    let (team_flat_share_id, team_consent_vote_id) =
                        Self::request_consent_on_terms_of_agreement(
                            bounty_info.foundation(), // org that supervises the relevant bounty
                            application_under_review.terms_of_agreement(),
                        )?;
                    let new_application = application_under_review
                        .start_team_consent_petition(team_flat_share_id, team_consent_vote_id);
                    // insert into map because application.state() changed => application changed
                    let new_state = new_application.state();
                    <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                    Ok(new_state)
                } else {
                    Ok(application_under_review.state())
                }
            }
            // TODO: clean up the outer_flat_share_id dispatched for team consent if NOT formally approved
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                wrapped_share_id,
                wrapped_vote_id,
            ) => {
                // check the vote status
                let status = Self::check_vote_status(wrapped_vote_id)?;
                if status {
                    let newly_registered_team_id = Self::approve_grant_to_register_team(
                        bounty_info.foundation(),
                        wrapped_share_id.into(),
                        application_under_review.terms_of_agreement(),
                    )?;
                    let new_application =
                        application_under_review.approve_grant(newly_registered_team_id);
                    let new_state = new_application.state();
                    <BountyApplications<T>>::insert(bounty_id, application_id, new_application);
                    Ok(new_state)
                } else {
                    Ok(application_under_review.state())
                }
            }
            //
            _ => Ok(application_under_review.state()),
        }
    }
}

impl<T: Trait> SubmitMilestone<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type MilestoneState = u32; // TODO: change
    fn submit_milestone(
        caller: T::AccountId, // must be from the team, maybe check sudo || flat_org_member
        bounty_id: u32,
        application_id: u32,
        team_id: Self::TeamId,
        submission_reference: IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<u32, DispatchError> { // returns milestone_id
        // check that the application is in the right state

        // check that the team_id corresponds to this application's state

        // check that the caller is a member of the TeamId flat shares

        // form the milestone

        // submit the milestone
        todo!()
    }
    fn trigger_milestone_review(
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the milestone

        // check that it is in a valid state to trigger a review

        // trigger the review and return the VoteId in the review somehow
        todo!()
    }
    // someone can try to call this and only the sudo can push things through at whim
    fn sudo_approves_milestone(
        caller: T::AccountId,
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the milestone

        // check that it is in a valid state to approve

        // approve and return the MilestoneState which should contain a transfer_id
        todo!()
    }
    fn poll_milestone(
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError> {
        // get the milestone

        // match on the state
        // -> if there is a review, check the result of it
        // --> if it passes, approve and return the MilestoneState which should contain a transfer_id
        todo!()
    }
}
