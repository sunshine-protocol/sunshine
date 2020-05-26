#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
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
    bank::{BankMapID, OnChainTreasuryID},
    bounty::{
        ApplicationState, BountyInformation, BountyMapID, BountyPaymentTracker, GrantApplication,
        MilestoneSubmission, ReviewBoard,
    },
    organization::{ShareID, TermsOfAgreement},
    traits::{
        ApplyVote, ApproveGrantApplication, BankDepositsAndSpends, BankReservations, BankSpends,
        BankStorageInfo, CheckBankBalances, CheckVoteStatus, CreateBounty, DepositIntoBank,
        EmpowerWithVeto, FoundationParts, GenerateUniqueID, GenerateUniqueKeyID,
        GetInnerOuterShareGroups, GetPetitionStatus, GetVoteOutcome, IDIsAvailable, MintableSignal,
        OnChainBank, OpenPetition, OpenShareGroupVote, OrgChecks, OrganizationDNS,
        OwnershipProportionCalculations, RegisterBankAccount, RegisterFoundation,
        RegisterShareGroup, RequestChanges, SeededGenerateUniqueID, ShareGroupChecks, SignPetition,
        SubmitGrantApplication, SupervisorPermissions, TermSheetExit, UpdatePetition,
        VoteOnProposal, WeightedShareIssuanceWrapper, WeightedShareWrapper,
    },
    uuid::UUID3,
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
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    /// This type wraps `membership`, `shares-membership`, and `shares-atomic`
    /// - it MUST be the same instance inherited by the bank module associated type
    type Organization: OrgChecks<u32, Self::AccountId>
        + ShareGroupChecks<u32, Self::AccountId>
        + GetInnerOuterShareGroups<u32, Self::AccountId>
        + SupervisorPermissions<u32, Self::AccountId>
        + WeightedShareWrapper<u32, u32, Self::AccountId>
        + WeightedShareIssuanceWrapper<u32, u32, Self::AccountId, Permill>
        + RegisterShareGroup<u32, u32, Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<u32, Self::AccountId, IpfsReference>;

    // TODO: start with spending functionality with balances for milestones
    // - then extend to offchain bank interaction (try to mirror logic/calls)
    type Bank: IDIsAvailable<OnChainTreasuryID>
        + IDIsAvailable<(OnChainTreasuryID, BankMapID)>
        + GenerateUniqueKeyID<(OnChainTreasuryID, BankMapID)>
        + GenerateUniqueID<OnChainTreasuryID>
        + OnChainBank
        + RegisterBankAccount<Self::AccountId, BalanceOf<Self>>
        + OwnershipProportionCalculations<Self::AccountId, BalanceOf<Self>, Permill>
        + BankDepositsAndSpends<BalanceOf<Self>>
        + CheckBankBalances<BalanceOf<Self>>
        + DepositIntoBank<Self::AccountId, IpfsReference, BalanceOf<Self>>
        + BankReservations<Self::AccountId, BalanceOf<Self>, IpfsReference>
        + BankSpends<Self::AccountId, BalanceOf<Self>>
        + BankStorageInfo<Self::AccountId, BalanceOf<Self>>
        + TermSheetExit<Self::AccountId, BalanceOf<Self>>;

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VotePetition: IDIsAvailable<UUID3>
        + GenerateUniqueID<UUID3>
        + GetPetitionStatus
        + EmpowerWithVeto<Self::AccountId>
        + OpenPetition<IpfsReference, Self::BlockNumber>
        + SignPetition<Self::AccountId, IpfsReference>
        + RequestChanges<Self::AccountId, IpfsReference>
        + UpdatePetition<Self::AccountId, IpfsReference>;

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VoteYesNo: IDIsAvailable<UUID3>
        + GenerateUniqueID<UUID3>
        + MintableSignal<Self::AccountId, Self::BlockNumber, Permill>
        + GetVoteOutcome
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
        ApplicationMustBeSubmittedAwaitingResponseToTriggerReview,
        AccountNotAuthorizedToTriggerApplicationReview,
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

        // TODO: helper method for getting all the bounties for an organization
        FoundationSponsoredBounties get(fn foundation_sponsored_bounties): map
            hasher(opaque_blake2_256) BountyId => Option<BountyInformation<IpfsReference, BalanceOf<T>>>;

        // second key is an ApplicationId
        BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) BountyId,
            hasher(opaque_blake2_256) u32 => Option<GrantApplication<T::AccountId, BalanceOf<T>, IpfsReference>>;

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
            acceptance_committee: ReviewBoard,
            supervision_committee: Option<ReviewBoard>,
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
        acceptance_committee: ReviewBoard,
    ) -> bool {
        match acceptance_committee {
            ReviewBoard::SimpleFlatReview(org_id, share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<
                    u32, T::AccountId
                >>::check_membership_in_share_group(org_id, ShareID::Flat(share_id).into(), account)
            },
            ReviewBoard::WeightedThresholdReview(org_id, share_id, _) => {
                <<T as Trait>::Organization as ShareGroupChecks<
                    u32, T::AccountId
                >>::check_membership_in_share_group(org_id, ShareID::WeightedAtomic(share_id).into(), account)
            },
        }
    }
    fn dispatch_application_review(
        committee: ReviewBoard,
        topic: IpfsReference,
        required_support: u32,
        required_against: Option<u32>,
        duration: Option<BlockNumber>,
    ) -> Result<u32, DispatchError> {
        let new_vote_id = match committee {
            ReviewBoard::SimpleFlatReview(org_id, share_id) => {
                // dispatch single approval vote (any vetos possible) in petition
                let id: u32 = <<T as Trait>::VotePetition as OpenPetition<IpfsReference, T::BlockNumber>>::open_petition(
                    org_id,
                    share_id,
                    topic,
                    required_support,
                    required_against,
                    duration
                )?.into();
                Ok(id)
            },
            ReviewBoard::WeightedThresholdReview(org_id, share_id, threshold) => {
                // dispatch weighted threshold review in vote-yesno
            },
        };
        Ok(new_vote_id)
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
}

impl<T: Trait> RegisterFoundation<BalanceOf<T>, T::AccountId> for Module<T> {
    // helper method to quickly bootstrap an organization form a donation
    // -> it should register an on-chain bank account and return the on-chain bank account identifier
    // TODO
    fn register_foundation_from_donation_deposit(
        from: T::AccountId,
        for_org: Self::OrgId,
        amount: BalanceOf<T>,
    ) -> Result<Self::BankId, DispatchError> {
        todo!()
    }
    fn register_foundation_from_existing_bank(
        org: Self::OrgId,
        bank: Self::BankId,
    ) -> DispatchResult {
        ensure!(
            <<T as Trait>::Bank as RegisterBankAccount<T::AccountId, BalanceOf<T>>>::check_bank_owner(bank.into(), org.into()),
            Error::<T>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        RegisteredFoundations::insert(org, bank, true);
        Ok(())
    }
}

impl<T: Trait> CreateBounty<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type BountyInfo = BountyInformation<IpfsReference, BalanceOf<T>>;
    // smpl vote config for this module in particular
    type ReviewCommittee = ReviewBoard;
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
            amount_claimed_available.clone() >= T::BountyLowerBound::get(),
            Error::<T>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        ensure!(
            Self::collateral_satisfies_module_limits(
                amount_reserved_for_bounty.clone(),
                amount_claimed_available.clone(),
            ),
            Error::<T>::BountyCollateralRatioMustMeetModuleRequirements
        );

        // reserve `amount_reserved_for_bounty` here by calling into `bank-onchain`
        let spend_reservation_id = <<T as Trait>::Bank as BankReservations<
            T::AccountId,
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
    type GrantApp = GrantApplication<T::AccountId, BalanceOf<T>, IpfsReference>;
    type TermsOfAgreement = TermsOfAgreement<T::AccountId>;
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

impl<T: Trait> ApproveGrantApplication<BalanceOf<T>, T::AccountId, IpfsReference> for Module<T> {
    type SupportedVoteMechanisms = u32;
    type AppState = ApplicationState;
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

        // TODO: look into the syntax fo dispatching a simple vote here

        // change the application status on the Applications

        // insert the new status into the Applications map
        Ok(application_to_review.state())
    }
    fn poll_application(
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError> {
        // get the bounty information
        let bounty_info = <FoundationSponsoredBounties<T>>::get(bounty_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfBountyDNE)?;
        // get the application information
        let application_to_review = <BountyApplications<T>>::get(bounty_id, application_id)
            .ok_or(Error::<T>::CannotReviewApplicationIfApplicationDNE)?;
        // TODO: if state is under review, check status and push it along if it has passed
        Ok(application_to_review.state())
    }
}
