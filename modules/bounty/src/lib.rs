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
    bank::{OffChainTreasuryID, OnChainTreasuryID},
    bounty::{
        BountyInformation, BountyPaymentTracker, GrantApplication, Milestone, MilestoneReview,
        MilestoneSchedule, ReviewBoard,
    },
    organization::{FormedOrganization, TermsOfAgreement},
    traits::{
        AccessGenesis, ApplyVote, ChangeBankBalances, ChangeGroupMembership, CheckBankBalances,
        CheckVoteStatus, CreateBounty, DepositWithdrawalOps, EmpowerWithVeto, GenerateUniqueID,
        GetDepositsByAccountForBank, GetInnerOuterShareGroups, GetPetitionStatus, GetVoteOutcome,
        IDIsAvailable, MintableSignal, OffChainBank, OnChainBank, OnChainWithdrawalFilters,
        OpenPetition, OpenShareGroupVote, OrgChecks, OrganizationDNS, RegisterOffChainBankAccount,
        RegisterOnChainBankAccount, RegisterShareGroup, RequestChanges, ShareGroupChecks,
        SignPetition, SupervisorPermissions, SupportedOrganizationShapes, UpdatePetition,
        VerifyOwnership, VoteOnProposal, WeightedShareIssuanceWrapper, WeightedShareWrapper,
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
        + GenerateUniqueID<OnChainTreasuryID>
        + IDIsAvailable<OffChainTreasuryID>
        + GenerateUniqueID<OffChainTreasuryID>
        + RegisterOffChainBankAccount
        + RegisterOnChainBankAccount<Self::AccountId, BalanceOf<Self>, Permill>
        + ChangeBankBalances<BalanceOf<Self>, Permill>
        + CheckBankBalances<Self::AccountId, BalanceOf<Self>, Permill>
        + OffChainBank
        + SupportedOrganizationShapes
        + OnChainBank<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>
        + GetDepositsByAccountForBank<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>
        + OnChainWithdrawalFilters<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>;

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
        PlaceHolder(AccountId), // delete this
        BountyPosted(AccountId, OrgId, BountyId, Currency),
        GrantApplicationSubmitted(AccountId, OrgId, BountyId, Currency),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PlaceHolderError,
        NoBankExistsAtInputTreasuryIdForCreatingBounty,
        WithdrawalPermissionsOfBankMustAlignWithCallerToUseForBounty,
        OrganizationBankDoesNotHaveEnoughBalanceToCreateBounty,
        BountyAmountBelowGlobalMinimum,
        BountyCollateralRatioBelowGlobalMinimum,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        pub PlaceHolderStorageValue get(fn place_holder_storage_value): u32;

        FoundationBountyNonce get(fn foundation_bounty_nonce): map
            hasher(opaque_blake2_256) FormedOrganization => BountyId;

        // TODO: ensure that the FormedOrganization is the controller for the bank account during registration of bounty
        FoundationSponsoredBounties get(fn foundation_sponsored_bounties): double_map
            hasher(opaque_blake2_256) FormedOrganization,
            hasher(opaque_blake2_256) BountyId => Option<BountyInformation<IpfsReference, BalanceOf<T>, T::AccountId>>;

        // TODO: push notifications should ping all supervisors when this submission occurs
        // - optionally: request their acknowledgement before starting the supervision vote according to the ReviewBoard (could be part of the ReviewBoard)
        MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) (FormedOrganization, BountyId),
            hasher(opaque_blake2_256) Milestone<IpfsReference, BalanceOf<T>> => Option<MilestoneReview>;

        BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) (FormedOrganization, BountyId),
            hasher(opaque_blake2_256) GrantApplication<T::AccountId, BalanceOf<T>, IpfsReference> => bool;
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

        #[weight = 0]
        fn create_bounty(
            origin,
            description: IpfsReference,
            bank_account: OnChainTreasuryID,
            amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
            amount_claimed_available: BalanceOf<T>,  // claimed available amount, not necessarily liquid
            acceptance_committee: ReviewBoard<T::AccountId>,
            supervision_committee: Option<ReviewBoard<T::AccountId>>,
        ) -> DispatchResult {
            // permissions, might be the first organization for example but it is permissioned
            let _ = ensure_signed(origin)?;
            Ok(())
        }
    }
}

impl<T: Trait> IDIsAvailable<(FormedOrganization, BountyId)> for Module<T> {
    fn id_is_available(id: (FormedOrganization, BountyId)) -> bool {
        <FoundationSponsoredBounties<T>>::get(id.0, id.1).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<(FormedOrganization, BountyId)> for Module<T> {
    fn generate_unique_id(
        proposed_id: (FormedOrganization, BountyId),
    ) -> (FormedOrganization, BountyId) {
        if !Self::id_is_available(proposed_id) {
            let mut id_counter = FoundationBountyNonce::get(proposed_id.0) + 1;
            while !Self::id_is_available((proposed_id.0, id_counter)) {
                id_counter += 1;
            }
            FoundationBountyNonce::insert(proposed_id.0, id_counter);
            (proposed_id.0, id_counter)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> SupportedOrganizationShapes for Module<T> {
    type FormedOrgId = <<T as Trait>::Bank as SupportedOrganizationShapes>::FormedOrgId;
}

impl<T: Trait> CreateBounty<IpfsReference, BalanceOf<T>> for Module<T> {
    type BankId = <<T as Trait>::Bank as RegisterOnChainBankAccount<
        <T as frame_system::Trait>::AccountId,
        BalanceOf<T>,
        Permill,
    >>::TreasuryId;
    type ReviewCommittee = ReviewBoard<T::AccountId>;
    fn screen_bounty_submission(
        caller: Self::FormedOrgId,
        description: IpfsReference,
        bank_account: Self::BankId,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> DispatchResult {
        Ok(())
    }
    // This method is still permissioned by the caller in terms of the actual caller's AccountId's relationship
    // to the caller: Self::FormedOrgId
    fn create_bounty(
        caller: Self::FormedOrgId,
        description: IpfsReference,
        bank_account: Self::BankId,
        amount_reserved_for_bounty: BalanceOf<T>, // collateral requirement
        amount_claimed_available: BalanceOf<T>, // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<(Self::FormedOrgId, u32), DispatchError> {
        // get bank state with bank id
        let bank_state = <<T as Trait>::Bank as CheckBankBalances<
            <T as frame_system::Trait>::AccountId,
            BalanceOf<T>,
            Permill,
        >>::get_bank(bank_account.clone())
        .ok_or(Error::<T>::NoBankExistsAtInputTreasuryIdForCreatingBounty)?;
        // verify that WithdrawalPermissions conform to shape of the `caller: FormedOrgId`
        ensure!(
            bank_state.verify_ownership(caller),
            Error::<T>::WithdrawalPermissionsOfBankMustAlignWithCallerToUseForBounty
        );
        // get the bank balance with bank id
        // TODO: I want an event emitted with this information somewhere
        let bank_balance = <<T as Trait>::Bank as CheckBankBalances<
            <T as frame_system::Trait>::AccountId,
            BalanceOf<T>,
            Permill,
        >>::get_bank_total_balance(bank_account)
        .ok_or(Error::<T>::OrganizationBankDoesNotHaveEnoughBalanceToCreateBounty)?;
        // ensure that the bank has more than the amount that needs to be reserved for the bounty
        ensure!(
            bank_balance >= amount_reserved_for_bounty,
            Error::<T>::OrganizationBankDoesNotHaveEnoughBalanceToCreateBounty
        );
        // TODO: set aside that amount for a reserved spend aka reserve this
        // amount for only withdrawal requests pertaining to this bounty?
        // - could just shift it from SAVINGS to RESERVED_FOR_SPENDS if its available
        // - but we also need to share context so that the money can flow when the acceptance
        // committee approves a grant && the supervising committee approves a milestone submission

        // check that the bounty amount is above the global
        ensure!(
            amount_claimed_available >= T::BountyLowerBound::get(),
            Error::<T>::BountyAmountBelowGlobalMinimum
        );
        let collateral_ratio = Permill::from_rational_approximation(
            amount_reserved_for_bounty,
            amount_claimed_available,
        );
        // check that the collateralization ratio is above the global (create helper)
        ensure!(
            collateral_ratio >= T::MinimumBountyCollateralRatio::get(),
            Error::<T>::BountyCollateralRatioBelowGlobalMinimum
        );

        // check existence of the acceptance committee
        // - must be an inner share group of the organization?

        // check existence of the supervision committee
        // - must be an inner share group of the organization?

        // generate unique identifier for bounty

        // form bounty

        // insert bounty into storage

        // return unique storage identifier
        Err(Error::<T>::PlaceHolderError.into())
    }
}
