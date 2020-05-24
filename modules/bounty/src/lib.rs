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
        BountyInformation, BountyPaymentTracker, GrantApplication, Milestone, MilestoneReview,
        MilestoneSchedule, ReviewBoard,
    },
    organization::TermsOfAgreement,
    traits::{
        AccessGenesis, ApplyVote, BankDepositsAndSpends, BankReservations, BankSpends,
        BankStorageInfo, ChangeGroupMembership, CheckBankBalances, CheckVoteStatus, CreateBounty,
        DepositIntoBank, EmpowerWithVeto, FoundationParts, GenerateUniqueID, GenerateUniqueKeyID,
        GetInnerOuterShareGroups, GetPetitionStatus, GetVoteOutcome, IDIsAvailable, MintableSignal,
        OnChainBank, OpenPetition, OpenShareGroupVote, OrgChecks, OrganizationDNS,
        OwnershipProportionCalculations, RegisterBankAccount, RegisterFoundation,
        RegisterOffChainBankAccount, RegisterShareGroup, RequestChanges, SeededGenerateUniqueID,
        ShareGroupChecks, SignPetition, SupervisorPermissions, SupportedOrganizationShapes,
        TermSheetExit, UpdatePetition, VoteOnProposal, WeightedShareIssuanceWrapper,
        WeightedShareWrapper,
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
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        FoundationBountyNonce get(fn foundation_bounty_nonce): map
            hasher(opaque_blake2_256) OrgId => BountyId;

        // unordered set for tracking foundations as relationships b/t OrgId and OnChainTreasuryID
        RegisteredFoundations get(fn registered_foundations): double_map
            hasher(blake2_128_concat) OrgId,
            hasher(blake2_128_concat) OnChainTreasuryID => bool;

        // TODO: helper method for getting all the bounties for an organization
        FoundationSponsoredBounties get(fn foundation_sponsored_bounties): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) BountyId => Option<BountyInformation<IpfsReference, BalanceOf<T>, T::AccountId>>;

        BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) (OrgId, BountyId),
            hasher(opaque_blake2_256) GrantApplication<T::AccountId, BalanceOf<T>, IpfsReference> => bool;

        MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) (OrgId, BountyId),
            hasher(opaque_blake2_256) Milestone<IpfsReference, BalanceOf<T>> => Option<MilestoneReview>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_bounty_on_behalf_of_foundation(
            origin,
            registered_organization: OrgId,
            description: IpfsReference,
            bank_account: OnChainTreasuryID,
            amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
            amount_claimed_available: BalanceOf<T>,  // claimed available amount, not necessarily liquid
            acceptance_committee: ReviewBoard<T::AccountId>,
            supervision_committee: Option<ReviewBoard<T::AccountId>>,
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
}

impl<T: Trait> IDIsAvailable<(OrgId, BountyId)> for Module<T> {
    fn id_is_available(id: (OrgId, BountyId)) -> bool {
        <FoundationSponsoredBounties<T>>::get(id.0, id.1).is_none()
    }
}

impl<T: Trait> SeededGenerateUniqueID<BountyId, OrgId> for Module<T> {
    fn generate_unique_id(seed: OrgId) -> BountyId {
        let mut id_counter = FoundationBountyNonce::get(seed) + 1;
        while !Self::id_is_available((seed, id_counter)) {
            id_counter += 1;
        }
        FoundationBountyNonce::insert(seed, id_counter);
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
    type BountyInfo = BountyInformation<IpfsReference, BalanceOf<T>, T::AccountId>;
    // smpl vote config for this module in particular
    type ReviewCommittee = ReviewBoard<T::AccountId>;
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
        // this constraints specifies required registration of the relationship between OrgId and OnChainBankId
        ensure!(
            RegisteredFoundations::get(foundation, bank_account),
            Error::<T>::FoundationMustBeRegisteredToCreateBounty
        );
        // enfore module constraints for all posted bounties
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
        let new_bounty_id = Self::generate_unique_id(foundation);
        // insert bounty_info object into storage
        <FoundationSponsoredBounties<T>>::insert(foundation, new_bounty_id, new_bounty_info);
        Ok(new_bounty_id)
    }
}
