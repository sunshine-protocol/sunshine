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
        OnChainTreasuryID,
        Sender,
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
    frame_system::Trait + org::Trait + vote::Trait + bank::Trait + court::Trait
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
        PlaceHolder(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        PlaceHolderError,
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
        pub FoundationSponsoredBounties get(fn foundation_sponsored_bounties): map
            hasher(opaque_blake2_256) T::BountyId => Option<
                BountyInformation<
                    T::IpfsReference,
                    OnChainTreasuryID,
                    BalanceOf<T>,
                    ResolutionMetadata<
                        T::OrgId,                  ThresholdConfig<T::Signal>,
                        T::BlockNumber,
                    >,
                >
            >;

        /// All bounty applications
        pub BountyApplications get(fn bounty_applications): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<GrantApplication<T::AccountId, T::OrgId, BalanceOf<T>, T::IpfsReference, ApplicationState<T::VoteId>>>;

        /// All milestone submissions
        pub MilestoneSubmissions get(fn milestone_submissions): double_map
            hasher(opaque_blake2_256) T::BountyId,
            hasher(opaque_blake2_256) T::BountyId => Option<MilestoneSubmission<T::AccountId, T::BountyId, T::IpfsReference, BalanceOf<T>, MilestoneStatus<T::VoteId, OnChainTreasuryID, T::TransferId>>>;

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
        <FoundationSponsoredBounties<T>>::get(id.id).is_none()
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
        BankTransfer<T::TransferId>,
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
        Sender<T::AccountId, OnChainTreasuryID>,
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
        poster: AccountId,
        on_behalf_of: Option<BankTransfer<T::TransferId>>,
        description: Hash,
        amount_reserved_for_bounty: Currency,
        acceptance_committee: ReviewCommittee,
        supervision_committee: Option<ReviewCommittee>,
    ) -> Result<Self::BountyId> {
        let new_poster = if let Some(bank_id) = on_behalf_of {
            // account is posting on behalf of bank so check org membership || org supervisor
            let bank = <bank::Module<T>>::bank_stores(bank_id)
                .ok_or(Error::<T>::CannotPostBountyOnBehalfOfBankThatDNE)?;
            let authentication =
                <org::Module<T>>::is_member_of_group(bank.org(), &poster)
                    || <org::Module<T>>::is_organization_supervisor(
                        bank.org(),
                        &poster,
                    );
            ensure!(authentication, Error::<T>::CannotPostBountiesOnBehalfOfOrgBasedOnAuthRequirements);

        // check that there is enough unclaimed from the transfer in question to reserve the funds from it by increasing the bank's reserves and increasing the amount claimed in the transfer state

        // reserve the amount officially
        } else {
            // TODO: use bank Currency and
            <T as court::Trait>::Currency::reserve(
                &poster,
                amount_reserved_for_bounty,
            )?;
            // the poster is posting as an individual
            Sender::Account(poster.clone())
        };
    }
}
