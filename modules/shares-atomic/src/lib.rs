#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::{
    share::{AtomicShareProfile, SimpleShareGenesis},
    traits::{
        AccessGenesis, ChainSudoPermissions, ChangeGroupMembership, GenerateUniqueID, GetGroupSize,
        GroupMembership, IDIsAvailable, LockableProfile, OrganizationSupervisorPermissions,
        ReservableProfile, ShareBank, SubGroupSupervisorPermissions, VerifyShape,
        WeightedShareGroup,
    },
    uuid::UUID2,
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
    traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{
        AtLeast32Bit, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Saturating, Zero,
    },
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};

pub type OrgId = u32;
pub type ShareId = u32;

pub trait Trait: system::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Must be synced in this module if the assigned type is an associated type for anything
    /// that inherits this module
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<OrgId>
        + GenerateUniqueID<OrgId>
        + ChainSudoPermissions<Self::AccountId>
        + OrganizationSupervisorPermissions<u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>;

    /// The ownership value for each member in the context of a (OrgId, ShareId)
    type Shares: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + CheckedSub
        + Zero;

    /// The hard limit on the number of times shares can be reserved
    type ReservationLimit: Get<u32>; // TODO: add softer limit that can be governed by supervisors
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait>::Shares,
    {
        /// Organization ID, Share Id, Account ID of reservee, times_reserved of their profile
        SharesReserved(OrgId, ShareId, AccountId, u32),
        /// Organization ID, Share Id, Account ID of unreservee, times_reserved of their profile
        SharesUnReserved(OrgId, ShareId, AccountId, u32),
        /// Organization ID, Share Id, Account Id
        SharesLocked(OrgId, ShareId, AccountId),
        /// Organization ID, Share Id, Account Id
        SharesUnlocked(OrgId, ShareId, AccountId),
        /// Organization ID, Share Id, Recipient AccountId, Issued Amount
        SharesIssued(OrgId, ShareId, AccountId, Shares),
        /// Organization ID, Share Id, Burned AccountId, Burned Amount
        SharesBurned(OrgId, ShareId, AccountId, Shares),
        /// Organization ID, Share Id, Total Shares Minted
        SharesBatchIssued(OrgId, ShareId, Shares),
        /// Organization ID, Share Id, Total Shares Burned
        SharesBatchBurned(OrgId, ShareId, Shares),
        /// Organization IDm Share Id, All Shares in Circulation
        TotalSharesIssued(OrgId, ShareId, Shares),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        LogicBugShouldBeCaughtInTests,
        UnAuthorizedRequestToSwapSupervisor,
        ReservationWouldExceedHardLimit,
        CannotUnreserveWithZeroReservations,
        ShareHolderMembershipUninitialized,
        ProfileNotInstantiated,
        CanOnlyBurnReservedShares,
        IssuanceCannotGoNegative,
        CannotIssueToLockedProfile,
        InitialIssuanceShapeIsInvalid,
        CantReserveMoreThanShareTotal,
        CannotBurnIfIssuanceDNE,
        OrganizationMustBeRegisteredToIssueShares,
        OrganizationMustBeRegisteredToBurnShares,
        OrganizationMustBeRegisteredToLockShares,
        OrganizationMustBeRegisteredToUnLockShares,
        OrganizationMustBeRegisteredToReserveShares,
        OrganizationMustBeRegisteredToUnReserveShares,
        NotAuthorizedToRegisterShares,
        NotAuthorizedToLockShares,
        NotAuthorizedToUnLockShares,
        NotAuthorizedToReserveShares,
        NotAuthorizedToUnReserveShares,
        NotAuthorizedToIssueShares,
        NotAuthorizedToBurnShares,
        CantBurnSharesIfReferenceCountIsNone,
        GenesisTotalMustEqualSumToUseBatchOps,
        IssuanceWouldOverflowShares,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Shares {
        /// The account that can do less stuff in the context of the share group for the organization
        ShareGroupSupervisor get(fn share_group_supervisor): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => Option<T::AccountId>;

        /// Share identity nonce for every org
        ShareIdCounter get(fn share_id_counter):
            map hasher(opaque_blake2_256) OrgId => ShareId;

        /// ShareIDs claimed set
        ClaimedShareIdentity get(fn claimed_share_identity): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => bool;

        /// Membership reference counter to see when an AccountId should be removed from an organization
        MembershipReferenceCounter get(fn membership_reference_counter): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) T::AccountId => u32;

        /// Total share issuance for the share type with `ShareId`
        /// also the main point of registration for (OrgId, ShareId) pairs (see `GenerateUniqueId`)
        TotalIssuance get(fn total_issuance): double_map hasher(opaque_blake2_256) OrgId, hasher(opaque_blake2_256) ShareId => Option<T::Shares>;

        /// The ShareProfile (set as an associated type for the module's Trait aka `DoubleStoredMap` #4820)
        Profile get(fn profile): double_map hasher(blake2_128_concat) UUID2, hasher(blake2_128_concat) T::AccountId => Option<AtomicShareProfile<T::Shares>>;

        /// The number of accounts in the share group
        ShareGroupSize get(fn share_group_size): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => u32;
    }
    add_extra_genesis {
        config(share_supervisors): Option<Vec<(OrgId, ShareId, T::AccountId)>>;
        /// All share allocations for all groups registered at genesis
        // OrgId, ShareId, AccountId, Share Amount
        config(shareholder_membership): Option<Vec<(OrgId, ShareId, T::AccountId, T::Shares)>>;

        build(|config: &GenesisConfig<T>| {
            if let Some(sup) = &config.share_supervisors {
                sup.iter().for_each(|(org, sid, acc)| {
                    ShareGroupSupervisor::<T>::insert(org, sid, acc);
                });
            }
            if let Some(mem) = &config.shareholder_membership {
                mem.iter().for_each(|(org_id, share_id, account, shares)| {
                    let share_supervisor = ShareGroupSupervisor::<T>::get(org_id, share_id).expect("share supervisor must exist in order to add members at genesis");
                    <Module<T>>::issue_shares(
                        T::Origin::from(Some(share_supervisor).into()),
                        *org_id,
                        *share_id,
                        account.clone(),
                        *shares,
                    ).expect("genesis member could not be added to the organization");
                });
            }
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        const ReservationLimit: u32 = T::ReservationLimit::get();

        #[weight = 0]
        fn issue_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId, shares: T::Shares) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for issuance
            let authentication: bool = Self::check_if_sudo_account(&issuer)
                                    || Self::check_if_organization_supervisor_account(organization, &issuer)
                                    || Self::is_sub_group_supervisor(organization, share_id, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToIssueShares);

            Self::issue(organization, share_id, who.clone(), shares, false)?;
            Self::deposit_event(RawEvent::SharesIssued(organization, share_id, who, shares));
            Ok(())
        }

        #[weight = 0]
        fn burn_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId, shares: T::Shares) -> DispatchResult {
            let burner = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToBurnShares);
            // second check is that this is an authorized party for burning
            let authentication: bool = Self::check_if_sudo_account(&burner)
                                    || Self::check_if_organization_supervisor_account(organization, &burner);
            ensure!(authentication, Error::<T>::NotAuthorizedToBurnShares);

            Self::burn(organization, share_id, who.clone(), shares, false)?;
            Self::deposit_event(RawEvent::SharesBurned(organization, share_id, who, shares));
            Ok(())
        }

        #[weight = 0]
        fn batch_issue_shares(origin, organization: OrgId, share_id: ShareId, new_accounts: Vec<(T::AccountId, T::Shares)>) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for issuance
            let authentication: bool = Self::check_if_sudo_account(&issuer)
                                    || Self::check_if_organization_supervisor_account(organization, &issuer)
                                    || Self::is_sub_group_supervisor(organization, share_id, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToIssueShares);
            let genesis: SimpleShareGenesis<T::AccountId, T::Shares> = new_accounts.into();
            let total_new_shares_minted = genesis.total();
            Self::batch_issue(organization, share_id, genesis)?;
            Self::deposit_event(RawEvent::SharesBatchIssued(organization, share_id, total_new_shares_minted));
            Ok(())
        }

        #[weight = 0]
        fn batch_burn_shares(origin, organization: OrgId, share_id: ShareId, old_accounts: Vec<(T::AccountId, T::Shares)>) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for burning
            let authentication: bool = Self::check_if_sudo_account(&issuer)
                                    || Self::check_if_organization_supervisor_account(organization, &issuer)
                                    || Self::is_sub_group_supervisor(organization, share_id, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToBurnShares);
            let genesis: SimpleShareGenesis<T::AccountId, T::Shares> = old_accounts.into();
            let total_new_shares_burned = genesis.total();
            Self::batch_burn(organization, share_id, genesis)?;
            Self::deposit_event(RawEvent::SharesBatchBurned(organization, share_id, total_new_shares_burned));
            Ok(())
        }

        #[weight = 0]
        fn lock_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId) -> DispatchResult {
            let locker = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToLockShares);
            // second check is that this is an authorized party for locking shares
            let authentication: bool = Self::check_if_sudo_account(&locker)
                                    || Self::check_if_organization_supervisor_account(organization, &locker)
                                    || Self::is_sub_group_supervisor(organization, share_id, &locker)
                                    || locker == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToLockShares);

            Self::lock_profile(organization, share_id, &who)?;
            Self::deposit_event(RawEvent::SharesLocked(organization, share_id, who));
            Ok(())
        }

        #[weight = 0]
        fn unlock_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId) -> DispatchResult {
            let unlocker = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToUnLockShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::check_if_sudo_account(&unlocker)
                                    || Self::check_if_organization_supervisor_account(organization, &unlocker)
                                    || Self::is_sub_group_supervisor(organization, share_id, &unlocker)
                                    || unlocker == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToUnLockShares);

            Self::unlock_profile(organization, share_id, &who)?;
            Self::deposit_event(RawEvent::SharesUnlocked(organization, share_id, who));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        #[weight = 0]
        fn reserve_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToReserveShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::check_if_sudo_account(&reserver)
                                    || Self::check_if_organization_supervisor_account(organization, &reserver)
                                    || Self::is_sub_group_supervisor(organization, share_id, &reserver)
                                    || reserver == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToReserveShares);


            let reservation_context = Self::reserve(organization, share_id, &who, None)?;
            let times_reserved = reservation_context.0;
            Self::deposit_event(RawEvent::SharesReserved(organization, share_id, who, times_reserved));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        #[weight = 0]
        fn unreserve_shares(origin, organization: OrgId, share_id: ShareId, who: T::AccountId) -> DispatchResult {
            let unreserver = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(Self::check_organization_existence(organization), Error::<T>::OrganizationMustBeRegisteredToUnReserveShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::check_if_sudo_account(&unreserver)
                                    || Self::check_if_organization_supervisor_account(organization, &unreserver)
                                    || Self::is_sub_group_supervisor(organization, share_id, &unreserver)
                                    || unreserver == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToUnReserveShares);

            let reservation_context = Self::unreserve(organization, share_id, &who, None)?;
            let times_reserved = reservation_context.0;
            Self::deposit_event(RawEvent::SharesUnReserved(organization, share_id, who, times_reserved));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    /// Set the ShareProfile
    fn set_profile(
        prefix_key: UUID2,
        who: &T::AccountId,
        new: &AtomicShareProfile<T::Shares>,
    ) -> DispatchResult {
        Profile::<T>::insert(prefix_key, who, new);
        Ok(())
    }

    // $$$ AUTH CHECKS $$$
    fn check_if_account_is_member_in_organization(
        organization: OrgId,
        account: &T::AccountId,
    ) -> bool {
        <<T as Trait>::OrgData as GroupMembership<<T as frame_system::Trait>::AccountId>>::is_member_of_group(organization, account)
    }
    fn check_organization_existence(organization: OrgId) -> bool {
        !<<T as Trait>::OrgData as IDIsAvailable<OrgId>>::id_is_available(organization)
    }
    fn check_if_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as ChainSudoPermissions<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
    }
    fn check_if_organization_supervisor_account(organization: OrgId, who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::is_organization_supervisor(organization, who)
    }
    /// Add Member
    fn add_new_member(organization: OrgId, new_member: T::AccountId) {
        <<T as Trait>::OrgData as ChangeGroupMembership<
            <T as frame_system::Trait>::AccountId,
        >>::add_group_member(organization, new_member, false)
    }
    /// Remove Member
    fn remove_old_member(organization: OrgId, old_member: T::AccountId) {
        <<T as Trait>::OrgData as ChangeGroupMembership<
            <T as frame_system::Trait>::AccountId,
        >>::remove_group_member(organization, old_member, false);
    }
}

impl<T: Trait> IDIsAvailable<UUID2> for Module<T> {
    fn id_is_available(id: UUID2) -> bool {
        !ClaimedShareIdentity::get(id.one(), id.two())
    }
}

impl<T: Trait> GenerateUniqueID<UUID2> for Module<T> {
    fn generate_unique_id(proposed_id: UUID2) -> UUID2 {
        if !Self::id_is_available(proposed_id) {
            let organization = proposed_id.one();
            let mut id_counter = ShareIdCounter::get(organization);
            while ClaimedShareIdentity::get(organization, id_counter) || id_counter == 0 {
                // TODO: add overflow check here
                id_counter += 1u32;
            }
            ShareIdCounter::insert(organization, id_counter);
            UUID2::new(organization, id_counter)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> SubGroupSupervisorPermissions<u32, u32, T::AccountId> for Module<T> {
    fn is_sub_group_supervisor(org: u32, sub_group: u32, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::share_group_supervisor(org, sub_group) {
            return who == &supervisor;
        }
        false
    }
    fn put_sub_group_supervisor(org: u32, sub_group: u32, supervisor: T::AccountId) {
        <ShareGroupSupervisor<T>>::insert(org, sub_group, supervisor)
    }
    fn set_sub_group_supervisor(
        org: u32,
        sub_group: u32,
        old_supervisor: &T::AccountId,
        new_supervisor: T::AccountId,
    ) -> DispatchResult {
        let authentication: bool = Self::check_if_sudo_account(&old_supervisor)
            || Self::check_if_organization_supervisor_account(org, &old_supervisor)
            || Self::is_sub_group_supervisor(org, sub_group, &old_supervisor);
        if authentication {
            <ShareGroupSupervisor<T>>::insert(org, sub_group, new_supervisor.clone());
            return Ok(());
        }
        Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
    }
}

impl<T: Trait> GetGroupSize for Module<T> {
    type GroupId = UUID2;

    fn get_size_of_group(group_id: Self::GroupId) -> u32 {
        ShareGroupSize::get(group_id.one(), group_id.two())
    }
}

impl<T: Trait> GroupMembership<T::AccountId> for Module<T> {
    fn is_member_of_group(group_id: Self::GroupId, who: &T::AccountId) -> bool {
        <Profile<T>>::get(group_id, who).is_some()
    }
}

impl<T: Trait> ReservableProfile<T::AccountId> for Module<T> {
    /// .0 is the number of times reserved and .1 is the amount reserved
    type ReservationContext = (u32, T::Shares);

    fn reserve(
        organization: OrgId,
        share_id: ShareId,
        who: &T::AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError> {
        let prefix_key = UUID2::new(organization, share_id);
        // rule: must have shares to reserve them!
        let old_profile =
            Profile::<T>::get(prefix_key, who).ok_or(Error::<T>::ProfileNotInstantiated)?;
        // TODO: allow multiple reservations for a single vote? this should be a layered config option
        let max_amount_is_share_total = old_profile.total();
        // THIS DESIGN ACCEPTS ANY RESERVE/UNRESERVE AS INCREMENTING/DECREMENTING TIMES_RESERVED IFF LESS_THAN TOTAL
        // TODO: if amt.0 > 1, iterate times_reserved by the number of times required; for now, we assume amt.0 == 1
        let amount_or_default = if let Some(amt) = amount {
            amt.1
        } else {
            max_amount_is_share_total
        };
        ensure!(
            max_amount_is_share_total >= amount_or_default,
            Error::<T>::CantReserveMoreThanShareTotal
        );
        let times_reserved_increment = if let Some(amt) = amount { amt.0 } else { 1u32 };
        let times_reserved = old_profile.times_reserved() + times_reserved_increment;
        // make sure it's below the hard reservation limit
        ensure!(
            times_reserved < T::ReservationLimit::get(),
            Error::<T>::ReservationWouldExceedHardLimit
        );
        // instantiate new share profile which just iterates times_reserved
        let new_share_profile = old_profile.iterate_times_reserved(times_reserved_increment);
        // set new share profile for `who`
        let new_times_reserved = new_share_profile.times_reserved();
        Self::set_profile(prefix_key, who, &new_share_profile)?;
        Ok((new_times_reserved, amount_or_default))
    }

    fn unreserve(
        organization: OrgId,
        share_id: ShareId,
        who: &T::AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError> {
        let prefix_key = UUID2::new(organization, share_id);
        // rule: must have shares to unreserve them!
        let old_profile =
            Profile::<T>::get(prefix_key, who).ok_or(Error::<T>::ProfileNotInstantiated)?;
        // TODO: allow multiple reservations for a single vote? this should be a layered config option
        let max_amount_is_share_total = old_profile.total();
        // THIS DESIGN ACCEPTS ANY RESERVE/UNRESERVE AS INCREMENTING/DECREMENTING TIMES_RESERVED IFF LESS_THAN TOTAL
        // TODO: if amt.0 > 1, iterate times_reserved by the number of times required; for now, we assume amt.0 == 1
        let amount_or_default = if let Some(amt) = amount {
            amt.1
        } else {
            max_amount_is_share_total
        };
        ensure!(
            max_amount_is_share_total >= amount_or_default,
            Error::<T>::CantReserveMoreThanShareTotal
        );
        let times_reserved_decrement = if let Some(amt) = amount {
            amt.0
        } else {
            // default decrement
            1u32
        };
        let current_times_reserved = old_profile.times_reserved();
        let new_times_reserved = current_times_reserved
            .checked_sub(times_reserved_decrement)
            .ok_or(Error::<T>::CannotUnreserveWithZeroReservations)?;
        // instantiate new share profile by incrementing times reserved
        let new_share_profile = old_profile.decrement_times_reserved(times_reserved_decrement);
        // set new share profile
        ensure!(
            new_times_reserved == new_share_profile.times_reserved(),
            Error::<T>::LogicBugShouldBeCaughtInTests
        );
        Self::set_profile(prefix_key, who, &new_share_profile)?;
        Ok((new_times_reserved, amount_or_default))
    }
}

impl<T: Trait> LockableProfile<T::AccountId> for Module<T> {
    fn lock_profile(organization: OrgId, share_id: ShareId, who: &T::AccountId) -> DispatchResult {
        let prefix_key = UUID2::new(organization, share_id);
        let locked_profile = if let Some(to_be_locked) = Profile::<T>::get(prefix_key, who) {
            to_be_locked.lock()
        } else {
            return Err(Error::<T>::ProfileNotInstantiated.into());
        };
        // lock the profile
        Profile::<T>::insert(prefix_key, who, locked_profile);
        Ok(())
    }

    fn unlock_profile(
        organization: OrgId,
        share_id: ShareId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let prefix_key = UUID2::new(organization, share_id);
        let locked_profile = if let Some(to_be_locked) = Profile::<T>::get(prefix_key, who) {
            to_be_locked.unlock()
        } else {
            return Err(Error::<T>::ProfileNotInstantiated.into());
        };
        // lock the profile
        Profile::<T>::insert(prefix_key, who, locked_profile);
        Ok(())
    }
}

impl<T: Trait> WeightedShareGroup<T::AccountId> for Module<T> {
    type Shares = T::Shares;
    type Profile = AtomicShareProfile<T::Shares>;
    type Genesis = SimpleShareGenesis<T::AccountId, T::Shares>;
    fn outstanding_shares(organization: OrgId, share_id: ShareId) -> Option<T::Shares> {
        <TotalIssuance<T>>::get(organization, share_id)
    }
    fn get_share_profile(
        organization: OrgId,
        share_id: ShareId,
        who: &T::AccountId,
    ) -> Option<Self::Profile> {
        let prefix_key = UUID2::new(organization, share_id);
        Profile::<T>::get(prefix_key, who)
    }
    fn shareholder_membership(organization: OrgId, share_id: ShareId) -> Option<Self::Genesis> {
        let prefix = UUID2::new(organization, share_id);
        // TODO: update once https://github.com/paritytech/substrate/pull/5335 is merged and pulled into local version
        if Self::id_is_available(prefix) {
            None
        } else {
            Some(
                <Profile<T>>::iter()
                    .filter(|(uuidtwo, _, _)| uuidtwo == &prefix)
                    .map(|(_, account, profile)| (account, profile.total()))
                    .collect::<Vec<(T::AccountId, T::Shares)>>()
                    .into(),
            )
        }
    }
}

impl<T: Trait> ShareBank<T::AccountId> for Module<T> {
    fn issue(
        organization: OrgId,
        share_id: ShareId,
        new_owner: T::AccountId,
        amount: T::Shares,
        batch: bool,
    ) -> DispatchResult {
        if !ClaimedShareIdentity::get(organization, share_id) {
            ClaimedShareIdentity::insert(organization, share_id, true);
        }
        // add new member to the organization if they are not already in it
        if !Self::check_if_account_is_member_in_organization(organization, &new_owner) {
            Self::add_new_member(organization, new_owner.clone());
        }
        // update the recipient's share profile
        let prefix_key = UUID2::new(organization, share_id);
        let old_share_profile = Profile::<T>::get(prefix_key, &new_owner);
        let new_share_profile = if let Some(old_profile) = old_share_profile {
            ensure!(
                old_profile.is_unlocked(),
                Error::<T>::CannotIssueToLockedProfile
            );
            old_profile.add_shares(amount)
        } else {
            // increase the MembershipReferenceCounter
            let new_share_group_count_for_account =
                <MembershipReferenceCounter<T>>::get(organization, &new_owner) + 1u32;
            <MembershipReferenceCounter<T>>::insert(
                organization,
                &new_owner,
                new_share_group_count_for_account,
            );
            AtomicShareProfile::new_shares(amount)
        };
        // update total issuance if not batch
        if !batch {
            let current_issuance =
                <TotalIssuance<T>>::get(organization, share_id).unwrap_or_else(T::Shares::zero);
            let new_amount = current_issuance.saturating_add(amount);
            <TotalIssuance<T>>::insert(organization, share_id, new_amount);
        }
        Profile::<T>::insert(prefix_key, &new_owner, new_share_profile);
        Ok(())
    }
    fn burn(
        organization: OrgId,
        share_id: ShareId,
        old_owner: T::AccountId,
        amount: T::Shares,
        batch: bool,
    ) -> DispatchResult {
        // (1) change total issuance
        let current_issuance = <TotalIssuance<T>>::get(organization, share_id)
            .ok_or(Error::<T>::CannotBurnIfIssuanceDNE)?;
        // (2) change owner's profile
        let prefix_key = UUID2::new(organization, share_id);
        let profile =
            Profile::<T>::get(prefix_key, &old_owner).ok_or(Error::<T>::ProfileNotInstantiated)?;
        // enforce invariant that the owner must have these shares to burn them
        let total_shares = &profile.total();
        ensure!(
            total_shares >= &amount,
            Error::<T>::CanOnlyBurnReservedShares
        );
        if !batch {
            ensure!(
                current_issuance >= amount,
                Error::<T>::IssuanceCannotGoNegative
            );
            let new_amount = current_issuance - amount;
            <TotalIssuance<T>>::insert(organization, share_id, new_amount);
        }
        // ..(2)
        let new_profile = profile.subtract_shares(amount);
        // if profile is empty, decrease the reference count
        if new_profile.is_zero() {
            let membership_rc = <MembershipReferenceCounter<T>>::get(organization, &old_owner)
                .checked_sub(1u32)
                .ok_or(Error::<T>::CantBurnSharesIfReferenceCountIsNone)?;
            if membership_rc == 0 {
                Self::remove_old_member(organization, old_owner.clone());
            }
            // update reference counter
            <MembershipReferenceCounter<T>>::insert(organization, &old_owner, membership_rc);
            // remove the profile associated with the prefix key, account_id
            Profile::<T>::remove(prefix_key, &old_owner);
        } else {
            Profile::<T>::insert(prefix_key, &old_owner, new_profile);
        }
        Ok(())
    }

    // pretty expensive, complexity linear with size of group
    fn batch_issue(organization: u32, share_id: u32, genesis: Self::Genesis) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance =
            <TotalIssuance<T>>::get(organization, share_id).unwrap_or_else(T::Shares::zero);
        let new_issuance = old_issuance
            .checked_add(&genesis.total())
            .ok_or(Error::<T>::IssuanceWouldOverflowShares)?;
        <TotalIssuance<T>>::insert(organization, share_id, new_issuance);
        genesis
            .account_ownership()
            .into_iter()
            .map(|(member, shares)| -> DispatchResult {
                Self::issue(organization, share_id, member, shares, true)
            })
            .collect::<DispatchResult>()?;
        Ok(())
    }
    // pretty expensive, complexity linear with size of group
    fn batch_burn(organization: u32, share_id: u32, genesis: Self::Genesis) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance =
            <TotalIssuance<T>>::get(organization, share_id).unwrap_or_else(T::Shares::zero);
        let new_issuance = old_issuance
            .checked_sub(&genesis.total())
            .ok_or(Error::<T>::IssuanceCannotGoNegative)?;
        <TotalIssuance<T>>::insert(organization, share_id, new_issuance);
        genesis
            .account_ownership()
            .into_iter()
            .map(|(member, shares)| -> DispatchResult {
                Self::burn(organization, share_id, member, shares, true)
            })
            .collect::<DispatchResult>()?;
        Ok(())
    }
}
