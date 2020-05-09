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
    traits::{
        ChangeGroupMembership, GenerateUniqueID, GetFlatShareGroup, GetGroupSize, GroupMembership,
        IDIsAvailable, SubSupervisorKeyManagement, SudoKeyManagement, SupervisorKeyManagement,
    },
    uuid::UUID2,
};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;

/// The organization identifier type
pub type OrgId = u32;
pub type ShareId = u32;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Must be synced in this module if the assigned type is an associated type for anything
    /// that inherits this module
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<OrgId>
        + GenerateUniqueID<OrgId>
        + SudoKeyManagement<Self::AccountId>
        + SupervisorKeyManagement<Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        /// Organization ID, New Member Account ID
        NewMemberAdded(OrgId, ShareId, AccountId),
        /// Organization ID, Old Member Account ID
        OldMemberRemoved(OrgId, ShareId, AccountId),
        /// Batch Addition by the Account ID
        BatchMemberAddition(AccountId, OrgId, ShareId),
        /// Batch Removal by the Account ID
        BatchMemberRemoval(AccountId, OrgId, ShareId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        UnAuthorizedSwapSudoRequest,
        NoExistingSudoKey,
        UnAuthorizedRequestToSwapSupervisor,
        NotAuthorizedToChangeMembership,
        ShareHolderGroupHasNoMembershipInStorage,
        CantBurnSharesIfReferenceCountIsNone,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as ShareMembership {
        /// The account that has supervisor privileges for this share class of the organization
        /// - still less power than organization's supervisor defined in `membership`
        OrganizationShareSupervisor get(fn organization_share_supervisor): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => Option<T::AccountId>;

        /// Identity nonce for registering organizations
        ShareIdCounter get(fn share_id_counter): map
            hasher(opaque_blake2_256) OrgId => ShareId;

        /// ShareIDs claimed set
        ClaimedShareIdentity get(fn claimed_share_identity): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => bool;

        /// Membership reference counter to see when an AccountId should be removed from an organization
        /// - upholds invariant that member must be a member of share group to stay a member of the organization
        MembershipReferenceCounter get(fn membership_reference_counter): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) T::AccountId => u32;

        /// The map to track organizational membership by share class
        ShareHolders get(fn share_holders): double_map hasher(blake2_128_concat) UUID2, hasher(blake2_128_concat) T::AccountId => bool;

        ShareGroupSize get(fn share_group_size): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId => u32;
    }
    add_extra_genesis {
        config(share_supervisors): Option<Vec<(OrgId, ShareId, T::AccountId)>>;
        /// The shareholder member definition at genesis, requires consistency with other module geneses (plural of genesis)
        config(shareholder_membership): Option<Vec<(OrgId, ShareId, T::AccountId, bool)>>;

        build(|config: &GenesisConfig<T>| {
            if let Some(sup) = &config.share_supervisors {
                sup.clone().iter().for_each(|(org, sid, acc)| {
                    OrganizationShareSupervisor::<T>::insert(org, sid, acc);
                });
            }
            if let Some(mem) = &config.shareholder_membership {
                mem.iter().for_each(|(org_id, share_id, account, _)| {
                    let share_supervisor = OrganizationShareSupervisor::<T>::get(org_id, share_id).expect("share supervisor must exist in order to add members at genesis");
                    <Module<T>>::add_new_member(
                        T::Origin::from(Some(share_supervisor).into()),
                        *org_id,
                        *share_id,
                        account.clone(),
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

        /// Add member to organization's share class
        #[weight = 0]
        fn add_new_member(
            origin,
            organization: OrgId,
            share_id: ShareId,
            new_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                                    || Self::check_if_organization_supervisor_account(organization, &caller)
                                    || Self::is_sub_organization_supervisor(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            let prefix = UUID2::new(organization, share_id);
            Self::add_group_member(prefix, new_member.clone(), false);
            Self::deposit_event(RawEvent::NewMemberAdded(organization, share_id, new_member));
            Ok(())
        }

        /// Remove member from organization's share class
        #[weight = 0]
        fn remove_old_member(
            origin,
            organization: OrgId,
            share_id: ShareId,
            old_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                                    || Self::check_if_organization_supervisor_account(organization, &caller)
                                    || Self::is_sub_organization_supervisor(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            let prefix = UUID2::new(organization, share_id);
            Self::remove_group_member(prefix, old_member.clone(), false);
            Self::deposit_event(RawEvent::OldMemberRemoved(organization, share_id, old_member));
            Ok(())
        }

        // Batch add members to organization's share class
        #[weight = 0]
        fn add_new_members(
            origin,
            organization: OrgId,
            share_id: ShareId,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                                    || Self::check_if_organization_supervisor_account(organization, &caller)
                                    || Self::is_sub_organization_supervisor(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            let prefix = UUID2::new(organization, share_id);
            Self::batch_add_group_members(prefix, new_members);
            Self::deposit_event(RawEvent::BatchMemberAddition(caller, organization, share_id));
            Ok(())
        }

        // Batch remove members from organization's share class
        #[weight = 0]
        fn remove_old_members(
            origin,
            organization: OrgId,
            share_id: ShareId,
            old_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                                    || Self::check_if_organization_supervisor_account(organization, &caller)
                                    || Self::is_sub_organization_supervisor(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            let prefix = UUID2::new(organization, share_id);
            Self::batch_remove_group_members(prefix, old_members);
            Self::deposit_event(RawEvent::BatchMemberRemoval(caller, organization, share_id));
            Ok(())
        }

        // Operations To Consider Adding:
        // - cas the membership set
        // - hard replace set with new set with no comparison check
    }
}

impl<T: Trait> Module<T> {
    // $$$ AUTH CHECKS $$$
    fn check_if_account_is_member_in_organization(
        organization: OrgId,
        account: &T::AccountId,
    ) -> bool {
        <<T as Trait>::OrgData as GroupMembership<<T as frame_system::Trait>::AccountId>>::is_member_of_group(organization, account)
    }
    fn check_if_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as SudoKeyManagement<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
    }
    fn check_if_organization_supervisor_account(organization: OrgId, who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as SupervisorKeyManagement<<T as frame_system::Trait>::AccountId>>::is_organization_supervisor(organization, who)
    }
    /// Add Member from Organization
    fn add_org_member(organization: OrgId, new_member: T::AccountId) {
        <<T as Trait>::OrgData as ChangeGroupMembership<
            <T as frame_system::Trait>::AccountId,
        >>::add_group_member(organization, new_member, false);
    }
    // // Remove Member from Organization
    fn remove_org_member(organization: OrgId, old_member: T::AccountId) {
        <<T as Trait>::OrgData as ChangeGroupMembership<
            <T as frame_system::Trait>::AccountId,
        >>::remove_group_member(organization, old_member, false);
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
        <ShareHolders<T>>::get(group_id, who)
    }
}

impl<T: Trait> IDIsAvailable<UUID2> for Module<T> {
    fn id_is_available(id: UUID2) -> bool {
        !ClaimedShareIdentity::get(id.one(), id.two())
    }
}

impl<T: Trait> GetFlatShareGroup<T::AccountId> for Module<T> {
    fn get_organization_share_group(organization: u32, share_id: u32) -> Option<Vec<T::AccountId>> {
        let prefix = UUID2::new(organization, share_id);
        // TODO: update once https://github.com/paritytech/substrate/pull/5335 is merged and pulled into local version
        if !Self::id_is_available(prefix) {
            Some(
                <ShareHolders<T>>::iter()
                    .filter(|(uuidtwo, _, _)| uuidtwo == &prefix)
                    .map(|(_, account, _)| account)
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    }
}

impl<T: Trait> GenerateUniqueID<UUID2> for Module<T> {
    fn generate_unique_id(proposed_id: UUID2) -> UUID2 {
        if !Self::id_is_available(proposed_id) {
            let static_org = proposed_id.one();
            let mut id_counter = <ShareIdCounter>::get(static_org) + 1u32;
            while ClaimedShareIdentity::get(static_org, id_counter) {
                // TODO: add overflow check here
                id_counter += 1u32;
            }
            <ShareIdCounter>::insert(static_org, id_counter);
            UUID2::new(static_org, id_counter)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> SubSupervisorKeyManagement<T::AccountId> for Module<T> {
    fn is_sub_organization_supervisor(uuid: u32, uuid2: u32, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::organization_share_supervisor(uuid, uuid2) {
            return who == &supervisor;
        }
        false
    }
    fn set_sub_supervisor(uuid: u32, uuid2: u32, who: T::AccountId) -> DispatchResult {
        <OrganizationShareSupervisor<T>>::insert(uuid, uuid2, who);
        Ok(())
    }
    fn swap_sub_supervisor(
        uuid: u32,
        uuid2: u32,
        old_key: T::AccountId,
        new_key: T::AccountId,
    ) -> Result<T::AccountId, DispatchError> {
        let authentication: bool = Self::check_if_sudo_account(&old_key)
            || Self::check_if_organization_supervisor_account(uuid, &old_key)
            || Self::is_sub_organization_supervisor(uuid, uuid2, &old_key);
        if authentication {
            <OrganizationShareSupervisor<T>>::insert(uuid, uuid2, new_key.clone());
            return Ok(new_key);
        }
        Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
    }
}

impl<T: Trait> ChangeGroupMembership<T::AccountId> for Module<T> {
    fn add_group_member(group_id: UUID2, new_member: T::AccountId, batch: bool) {
        let organization = group_id.one();
        let share_id = group_id.two();
        if !ClaimedShareIdentity::get(organization, share_id) {
            ClaimedShareIdentity::insert(organization, share_id, true);
        }
        if !Self::check_if_account_is_member_in_organization(organization, &new_member) {
            Self::add_org_member(organization, new_member.clone());
        }
        if !batch {
            let new_share_group_size: u32 = ShareGroupSize::get(organization, share_id) + 1u32;
            ShareGroupSize::insert(organization, share_id, new_share_group_size);
        }
        <ShareHolders<T>>::insert(group_id, &new_member, true);
        let new_membership_rc =
            <MembershipReferenceCounter<T>>::get(organization, &new_member) + 1u32;
        <MembershipReferenceCounter<T>>::insert(organization, new_member, new_membership_rc);
    }
    fn remove_group_member(group_id: UUID2, old_member: T::AccountId, batch: bool) {
        let organization = group_id.one();
        let share_id = group_id.two();
        if !batch {
            let new_share_group_size: u32 =
                ShareGroupSize::get(organization, share_id).saturating_sub(1u32);
            ShareGroupSize::insert(organization, share_id, new_share_group_size);
        }
        let membership_rc =
            <MembershipReferenceCounter<T>>::get(organization, &old_member).saturating_sub(1u32);
        // remove from share group
        <ShareHolders<T>>::insert(group_id, &old_member, false);
        // if rc is 0, remove from organization
        if membership_rc == 0 {
            Self::remove_org_member(organization, old_member.clone());
        }
        <MembershipReferenceCounter<T>>::insert(organization, &old_member, membership_rc);
    }
    fn batch_add_group_members(group_id: UUID2, new_members: Vec<T::AccountId>) {
        let organization = group_id.one();
        let share_id = group_id.two();
        let size_increase = new_members.len() as u32;
        let new_share_group_size: u32 = ShareGroupSize::get(organization, share_id) + size_increase;
        new_members.into_iter().for_each(|member| {
            // TODO: does this return a DispatchError if one of them errs? fallible iterators
            Self::add_group_member(group_id, member, true);
        });
        ShareGroupSize::insert(organization, share_id, new_share_group_size);
    }
    fn batch_remove_group_members(group_id: UUID2, old_members: Vec<T::AccountId>) {
        let organization = group_id.one();
        let share_id = group_id.two();
        let size_decrease = old_members.len() as u32;
        let new_share_group_size: u32 =
            ShareGroupSize::get(organization, share_id).saturating_sub(size_decrease);
        old_members.into_iter().for_each(|member| {
            Self::remove_group_member(group_id, member, true);
        });
        ShareGroupSize::insert(organization, share_id, new_share_group_size);
    }
}
