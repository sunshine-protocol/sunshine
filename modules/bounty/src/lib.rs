#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    traits::{Currency, Get},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchResult, Permill};
use sp_std::prelude::*;

use util::{
    bank::{OffChainTreasuryID, OnChainTreasuryID},
    bounty::{
        BountyInformation, BountyPaymentTracker, GrantApplication, Milestone, MilestoneReview,
        MilestoneSchedule,
    },
    organization::{FormedOrganization, TermsOfAgreement},
    traits::{
        AccessGenesis, ApplyVote, ChangeBankBalances, ChangeGroupMembership, CheckVoteStatus,
        DepositWithdrawalOps, EmpowerWithVeto, GenerateUniqueID, GetDepositsByAccountForBank,
        GetInnerOuterShareGroups, GetPetitionStatus, GetVoteOutcome, IDIsAvailable, MintableSignal,
        OffChainBank, OnChainBank, OnChainWithdrawalFilters, OpenPetition, OpenShareGroupVote,
        OrgChecks, OrganizationDNS, RegisterOffChainBankAccount, RegisterOnChainBankAccount,
        RegisterShareGroup, RequestChanges, ShareGroupChecks, SignPetition, SupervisorPermissions,
        SupportedOrganizationShapes, UpdatePetition, VoteOnProposal, WeightedShareIssuanceWrapper,
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
        + GenerateUniqueID<OnChainTreasuryID>
        + IDIsAvailable<OffChainTreasuryID>
        + GenerateUniqueID<OffChainTreasuryID>
        + RegisterOffChainBankAccount
        + RegisterOnChainBankAccount<Self::AccountId, BalanceOf<Self>, Permill>
        + ChangeBankBalances<BalanceOf<Self>, Permill>
        + OffChainBank
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
            hasher(opaque_blake2_256) BountyId => Option<BountyInformation<IpfsReference, BalanceOf<T>, Permill>>;

        // TODO: push notifications should ping all supervisors when this submission occurs
        // - optionally: request their acknowledgement before starting the supervision vote according to the VoteConfig (could be part of the VoteConfig)
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
    }
}
