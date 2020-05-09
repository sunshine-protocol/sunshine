#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]

mod tests;

use frame_support::{decl_error, decl_event, decl_module, decl_storage, traits::Currency};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchResult, Permill};
use sp_std::prelude::*;

use util::{
    bank::{OffChainTreasuryID, OnChainTreasuryID},
    bounty::{
        ApplicationId, BountyApplication, BountyId, BountyInformation, BountyPaymentTracker,
        Milestone, MilestoneId, MilestoneReview, MilestoneSchedule, TaskId,
    },
    organization::TermsOfAgreement,
    traits::{
        AccessGenesis, ApplyVote, ChangeBankBalances, ChangeGroupMembership, CheckVoteStatus,
        DepositWithdrawalOps, EmpowerWithVeto, GenerateUniqueID, GetDepositsByAccountForBank,
        GetFlatShareGroup, GetGroupSize, GetPetitionStatus, GetVoteOutcome, GroupMembership,
        IDIsAvailable, LockableProfile, MintableSignal, OffChainBank, OnChainBank,
        OnChainWithdrawalFilters, OpenPetition, OpenShareGroupVote, OrganizationDNS,
        RegisterOffChainBankAccount, RegisterOnChainBankAccount, RegisterShareGroup,
        RequestChanges, ReservableProfile, ShareBank, SignPetition, SubSupervisorKeyManagement,
        SudoKeyManagement, SupervisorKeyManagement, SupportedOrganizationShapes, UpdatePetition,
        VoteOnProposal, WeightedShareGroup,
    },
    uuid::{UUID, UUID2, UUID3, UUID4},
};

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The organization identfier
pub type OrgId = u32;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::WeightedShareData as WeightedShareGroup<
    <T as frame_system::Trait>::AccountId,
>>::Shares;
/// The balances type for this module
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    /// Used for permissions and shared for organizational membership in both
    /// shares modules
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<OrgId>
        + GenerateUniqueID<OrgId>
        + SudoKeyManagement<Self::AccountId>
        + SupervisorKeyManagement<Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>;

    /// Used for shares-membership -> vote-petition
    type FlatShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId, GroupId = UUID2>
        + IDIsAvailable<UUID2>
        + GenerateUniqueID<UUID2>
        + SubSupervisorKeyManagement<Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>
        + GetFlatShareGroup<Self::AccountId>;

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VotePetition: IDIsAvailable<UUID3>
        + GenerateUniqueID<UUID3>
        + GetPetitionStatus
        + EmpowerWithVeto<Self::AccountId>
        + OpenPetition<IpfsReference, Self::BlockNumber>
        + SignPetition<Self::AccountId, IpfsReference>
        + RequestChanges<Self::AccountId, IpfsReference>
        + UpdatePetition<Self::AccountId, IpfsReference>;

    /// Used for shares-atomic -> vote-yesno
    /// - this is NOT synced with FlatShareData
    /// so the `SharesOf<T>` and `ShareId` checks must be treated separately
    type WeightedShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<UUID2>
        + GenerateUniqueID<UUID2>
        + WeightedShareGroup<Self::AccountId>
        + ShareBank<Self::AccountId>
        + ReservableProfile<Self::AccountId>
        + LockableProfile<Self::AccountId>
        + SubSupervisorKeyManagement<Self::AccountId>;

    // TODO: use this when adding TRIGGER => VOTE => OUTCOME framework for util::bank::Spends
    type VoteYesNo: IDIsAvailable<UUID3>
        + GenerateUniqueID<UUID3>
        + MintableSignal<Self::AccountId, Self::BlockNumber, Permill>
        + GetVoteOutcome
        + OpenShareGroupVote<Self::AccountId, Self::BlockNumber, Permill>
        + ApplyVote
        + CheckVoteStatus
        + VoteOnProposal<Self::AccountId, IpfsReference, Self::BlockNumber, Permill>;

    // TODO: start with spending functionality with balances for milestones
    // - then extend to offchain bank interaction (try to mirror logic/calls)
    type Bank: IDIsAvailable<OnChainTreasuryID>
        + GenerateUniqueID<OnChainTreasuryID>
        + RegisterShareGroup<Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<Self::AccountId, IpfsReference>
        + RegisterOnChainBankAccount<Self::AccountId, BalanceOf<Self>, Permill>
        + ChangeBankBalances<BalanceOf<Self>, Permill>
        + OnChainBank<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>
        + GetDepositsByAccountForBank<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>
        + OnChainWithdrawalFilters<Self::AccountId, IpfsReference, BalanceOf<Self>, Permill>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Currency = BalanceOf<T>,
    {
        PlaceHolder(AccountId), // delete this
        BountyPosted(AccountId, OrgId, BountyId, Currency),
        BountyApplicationSubmitted(AccountId, OrgId, BountyId, ApplicationId, Currency),
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
