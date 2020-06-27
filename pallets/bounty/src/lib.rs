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
        FullBankId,
        OnChainTreasuryID,
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
        PostBounty,
        RegisterOrgAccount,
        RegisterOrganization,
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
        // BountyApplicationSubmitted(),
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
                    BankOrAccount<OnChainTreasuryID, T::AccountId>,
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
                    T::OrgId,
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
                    MilestoneStatus<T::VoteId, OnChainTreasuryID, T::BankId>
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

impl<T: Trait>
    PostBounty<
        T::AccountId,
        T::OrgId,
        BankSpend<FullBankId<T::BankId>>,
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
        BankOrAccount<OnChainTreasuryID, T::AccountId>,
        T::IpfsReference,
        BalanceOf<T>,
        ResolutionMetadata<
            T::OrgId,
            ThresholdConfig<T::Signal>,
            T::BlockNumber,
        >,
    >;
    type BountyId = T::BountyId;
    fn post_bounty(
        poster: T::AccountId,
        on_behalf_of: Option<BankSpend<FullBankId<T::BankId>>>,
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
        let bounty_poster: BankOrAccount<OnChainTreasuryID, T::AccountId> =
            if let Some(bank_spend) = on_behalf_of {
                match bank_spend {
                    BankSpend::Transfer(full_transfer_id) => {
                        ensure!(
                            <bank::Module<T>>::is_bank(full_transfer_id.id),
                            Error::<T>::CannotPostBountyIfBankReferencedDNE
                        );
                        let transfer_info = <bank::Module<T>>::transfer_info(full_transfer_id.id, full_transfer_id.sub_id).ok_or(Error::<T>::CannotPostBountyOnBehalfOfOrgWithInvalidTransferReference)?;
                        // ensure amount left is above amount_reserved_for_bounty; this check is aggressive because we don't really need to reserve until an application is approved
                        ensure!(transfer_info.amount_left() >= amount_reserved_for_bounty, Error::<T>::CannotPostBountyIfAmountExceedsAmountLeftFromSpendReference);
                        BankOrAccount::Bank(full_transfer_id.id)
                    }
                    BankSpend::Reserved(full_reservation_id) => {
                        ensure!(
                            <bank::Module<T>>::is_bank(full_reservation_id.id),
                            Error::<T>::CannotPostBountyIfBankReferencedDNE
                        );
                        let spend_reservation = <bank::Module<T>>::spend_reservations(full_reservation_id.id, full_reservation_id.sub_id).ok_or(Error::<T>::CannotPostBountyOnBehalfOfOrgWithInvalidSpendReservation)?;
                        // ensure amount left is above amount_reserved_for_bounty; this check is aggressive because we don't really need to reserve until an application is approved
                        ensure!(spend_reservation.amount_left() >= amount_reserved_for_bounty, Error::<T>::CannotPostBountyIfAmountExceedsAmountLeftFromSpendReference);
                        BankOrAccount::Bank(full_reservation_id.id)
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
