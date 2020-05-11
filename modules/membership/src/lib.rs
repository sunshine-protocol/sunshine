#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::traits::{
    ChainSudoPermissions, ChangeGroupMembership, GenerateUniqueID, GetGroupSize, GroupMembership,
    IDIsAvailable, OrganizationSupervisorPermissions,
};

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure};
use frame_system::{self as system, ensure_signed};
use sp_runtime::DispatchResult;
use sp_std::prelude::*;

pub type OrgId = u32;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        /// Organization ID, New Member Account ID
        NewMemberAdded(OrgId, AccountId),
        /// Organization ID, Old Member Account ID
        OldMemberRemoved(OrgId, AccountId),
        /// Batch Addition by the Account ID
        BatchMemberAddition(AccountId, OrgId),
        /// Batch Removal by the Account ID
        BatchMemberRemoval(AccountId, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        UnAuthorizedSwapSudoRequest,
        NoExistingSudoKey,
        UnAuthorizedRequestToSwapSupervisor,
        NotAuthorizedToChangeMembership,
        OrganizationHasNoMembershipInStorage,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Membership {
        /// The account that can set all the organization supervisors, should be replaced by committee-based governance
        SudoKey build(|config: &GenesisConfig<T>| Some(config.omnipotent_key.clone())): Option<T::AccountId>;
        /// The account that can do a lot of supervisor only stuff for the organization
        OrganizationSupervisor get(fn organization_supervisor):
            map hasher(opaque_blake2_256) OrgId => Option<T::AccountId>;

        /// Identity nonce for registering organizations
        OrgIdNonce get(fn org_id_counter): OrgId;

        /// For registering the OrgId
        ClaimedOrganizationIdentity get(fn claimed_organization_identity) build(
            |_: &GenesisConfig<T>| { // FOR ALL GENESIS REGARDLESS OF CONFIG
                let mut zeroth_org_claimed_at_genesis = Vec::<(OrgId, bool)>::new();
                zeroth_org_claimed_at_genesis.push((0u32, true));
                zeroth_org_claimed_at_genesis
        }): map hasher(opaque_blake2_256) OrgId => bool;

        /// The map to track organizational membership
        Members get(fn members): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) T::AccountId => bool;

        /// The size for each organization
        OrganizationSize get(fn organization_size): map hasher(opaque_blake2_256) OrgId => u32;
    }
    add_extra_genesis {
        /// The sudo key for managing setup at genesis
        config(omnipotent_key): T::AccountId;
        /// All organizational memberships registered at genesis
        config(membership): Option<Vec<(OrgId, T::AccountId, bool)>>;

        build(|config: &GenesisConfig<T>| {
            if let Some(mem) = &config.membership {
                mem.iter().for_each(|(org_id, account, _)| {
                    <Module<T>>::add_new_member(
                        T::Origin::from(Some(config.omnipotent_key.clone()).into()),
                        *org_id,
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

        /// Add member to organization
        #[weight = 0]
        fn add_new_member(
            origin,
            organization: OrgId,
            new_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::add_group_member(organization, new_member.clone(), false);
            Self::deposit_event(RawEvent::NewMemberAdded(organization, new_member));
            Ok(())
        }

        /// Remove member to organization
        #[weight = 0]
        fn remove_old_member(
            origin,
            organization: OrgId,
            old_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::remove_group_member(organization, old_member.clone(), false);
            Self::deposit_event(RawEvent::OldMemberRemoved(organization, old_member));
            Ok(())
        }

        // Batch add members to organization
        #[weight = 0]
        fn add_new_members(
            origin,
            organization: OrgId,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_add_group_members(organization, new_members);
            Self::deposit_event(RawEvent::BatchMemberAddition(caller, organization));
            Ok(())
        }

        // Batch remove members from organization
        #[weight = 0]
        fn remove_old_members(
            origin,
            organization: OrgId,
            old_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_remove_group_members(organization, old_members);
            Self::deposit_event(RawEvent::BatchMemberRemoval(caller, organization));
            Ok(())
        }

        // Operations To Consider Adding:
        // - cas the membership set
        // - hard replace set with new set with no comparison check
    }
}

impl<T: Trait> GetGroupSize for Module<T> {
    type GroupId = OrgId;

    fn get_size_of_group(group_id: Self::GroupId) -> u32 {
        OrganizationSize::get(group_id)
    }
}

impl<T: Trait> GroupMembership<T::AccountId> for Module<T> {
    fn is_member_of_group(group_id: Self::GroupId, who: &T::AccountId) -> bool {
        <Members<T>>::get(group_id, who)
    }
}

impl<T: Trait> IDIsAvailable<OrgId> for Module<T> {
    fn id_is_available(id: OrgId) -> bool {
        !ClaimedOrganizationIdentity::get(id)
    }
}

impl<T: Trait> GenerateUniqueID<OrgId> for Module<T> {
    fn generate_unique_id(proposed_id: OrgId) -> OrgId {
        if !Self::id_is_available(proposed_id) {
            let mut id_counter = OrgIdNonce::get() + 1u32;
            while ClaimedOrganizationIdentity::get(id_counter) {
                // TODO: add overflow check here
                id_counter += 1u32;
            }
            OrgIdNonce::put(id_counter);
            id_counter
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> ChainSudoPermissions<T::AccountId> for Module<T> {
    fn is_sudo_key(who: &T::AccountId) -> bool {
        if let Some(okey) = <SudoKey<T>>::get() {
            return who == &okey;
        }
        false
    }
    fn put_sudo_key(who: T::AccountId) {
        <SudoKey<T>>::put(who);
    }
    // only the sudo key can swap the sudo key (todo experiment: key recovery from some number of supervisors?)
    fn set_sudo_key(old_key: &T::AccountId, new_key: T::AccountId) -> DispatchResult {
        if let Some(okey) = <SudoKey<T>>::get() {
            if old_key == &okey {
                <SudoKey<T>>::put(new_key.clone());
                return Ok(());
            }
            return Err(Error::<T>::UnAuthorizedSwapSudoRequest.into());
        }
        Err(Error::<T>::NoExistingSudoKey.into())
    }
}

impl<T: Trait> OrganizationSupervisorPermissions<u32, T::AccountId> for Module<T> {
    fn is_organization_supervisor(org: u32, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::organization_supervisor(org) {
            return who == &supervisor;
        }
        false
    }
    // set the supervisor for the organization in whatever context
    fn put_organization_supervisor(org: u32, who: T::AccountId) {
        <OrganizationSupervisor<T>>::insert(org, who);
    }
    // sudo key and the current supervisor have the power to change the supervisor
    fn set_organization_supervisor(
        org: OrgId,
        old_supervisor: &T::AccountId,
        new_supervisor: T::AccountId,
    ) -> DispatchResult {
        let authentication: bool = Self::is_organization_supervisor(org, &old_supervisor)
            || Self::is_sudo_key(&old_supervisor);
        if authentication {
            <OrganizationSupervisor<T>>::insert(org, new_supervisor.clone());
            return Ok(());
        }
        Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
    }
}

impl<T: Trait> ChangeGroupMembership<T::AccountId> for Module<T> {
    fn add_group_member(organization: OrgId, new_member: T::AccountId, batch: bool) {
        if !batch {
            // TODO: check if this bug is everywhere like in the shares modules identity claim infrastructure
            // -- if the call to add members was changed to batch from the shares modules, this would need to be placed
            // outside this if statement
            if !ClaimedOrganizationIdentity::get(organization) {
                ClaimedOrganizationIdentity::insert(organization, true);
            }
            let new_organization_size = OrganizationSize::get(organization) + 1u32;
            OrganizationSize::insert(organization, new_organization_size);
        }
        <Members<T>>::insert(organization, new_member, true);
    }
    fn remove_group_member(organization: OrgId, old_member: T::AccountId, batch: bool) {
        if !batch {
            let new_organization_size = OrganizationSize::get(organization).saturating_sub(1u32);
            OrganizationSize::insert(organization, new_organization_size);
        }
        <Members<T>>::insert(organization, old_member, false);
    }
    fn batch_add_group_members(organization: OrgId, new_members: Vec<T::AccountId>) {
        let size_increase: u32 = new_members.len() as u32;
        // TODO: make this a saturating add to prevent overflow attack
        let new_organization_size: u32 =
            OrganizationSize::get(organization).saturating_add(size_increase);
        OrganizationSize::insert(organization, new_organization_size);
        new_members.into_iter().for_each(|member| {
            Self::add_group_member(organization, member, true);
        });
    }
    fn batch_remove_group_members(organization: OrgId, old_members: Vec<T::AccountId>) {
        let size_decrease: u32 = old_members.len() as u32;
        let new_organization_size: u32 =
            OrganizationSize::get(organization).saturating_sub(size_decrease);
        OrganizationSize::insert(organization, new_organization_size);
        old_members.into_iter().for_each(|member| {
            Self::remove_group_member(organization, member, true);
        });
    }
}
