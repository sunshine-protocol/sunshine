#![recursion_limit = "256"]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The org module is a shim for the membership modules for deeper inheritance

#[cfg(test)]
mod tests;

use util::{
    organization::{Organization, OrganizationSource},
    share::{ShareID, SimpleShareGenesis},
    traits::{
        AccessGenesis, AccessProfile, ChainSudoPermissions, ChangeGroupMembership,
        FlatShareWrapper, GenerateUniqueID, GetFlatShareGroup, GetGroupSize,
        GetInnerOuterShareGroups, GroupMembership, IDIsAvailable, LockableProfile, OrgChecks,
        OrganizationDNS, OrganizationSupervisorPermissions, PassShareIdDown, RegisterShareGroup,
        ReservableProfile, SeededGenerateUniqueID, ShareBank, ShareGroupChecks,
        SubGroupSupervisorPermissions, SupervisorPermissions, WeightedShareGroup,
        WeightedShareIssuanceWrapper, WeightedShareWrapper,
    },
    uuid::ShareGroup,
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
    Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    DispatchResult, Permill,
};
use sp_std::{fmt::Debug, prelude::*};

/// The weighted shares
// pub type SharesOf<T> = <<T as Trait>::WeightedShareData as WeightedShareGroup<
//     T::OrgId,
//     WeightedShareId<T>,
//     <T as frame_system::Trait>::AccountId,
// >>::Shares;
// /// The organization identifier type
// pub type T::OrgId = <<T as Trait>::OrgData as GetGroupSize>::GroupId;
// /// The flat share group identifier type
// pub type FlatShareId<T> = <<T as Trait>::FlatShareData as PassShareIdDown>::ShareId;
// /// The weighted share group identifier type
// pub type WeightedShareId<T> = <<T as Trait>::WeightedShareData as PassShareIdDown>::ShareId;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type IpfsReference: Parameter + Member;

    type OrgId: Parameter
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
    // should this be the same type as OrgId?
    type ShareId: Parameter
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

    // /// Used for shares-membership -> vote-petition
    // type FlatShareData: PassShareIdDown
    //     + GetGroupSize<GroupId = ShareGroup<OrgId<Self>, FlatShareId<Self>>>
    //     + GroupMembership<Self::AccountId, GroupId = ShareGroup<OrgId<Self>, FlatShareId<Self>>>
    //     + IDIsAvailable<ShareGroup<OrgId<Self>, FlatShareId<Self>>>
    //     + SeededGenerateUniqueID<FlatShareId<Self>, OrgId<Self>>
    //     + SubGroupSupervisorPermissions<OrgId<Self>, FlatShareId<Self>, Self::AccountId>
    //     + ChangeGroupMembership<Self::AccountId>
    //     + GetFlatShareGroup<OrgId<Self>, FlatShareId<Self>, Self::AccountId>;

    // /// Used for shares-atomic -> vote-yesno
    // /// - this is NOT synced with FlatShareData
    // /// so the `SharesOf<T>` and `ShareId` checks must be treated separately
    // type WeightedShareData: PassShareIdDown
    //     + GetGroupSize<GroupId = ShareGroup<OrgId<Self>, WeightedShareId<Self>>>
    //     + GroupMembership<Self::AccountId>
    //     + IDIsAvailable<ShareGroup<OrgId<Self>, WeightedShareId<Self>>>
    //     + SeededGenerateUniqueID<WeightedShareId<Self>, OrgId<Self>>
    //     + WeightedShareGroup<OrgId<Self>, WeightedShareId<Self>, Self::AccountId>
    //     + ShareBank<OrgId<Self>, WeightedShareId<Self>, Self::AccountId>
    //     + ReservableProfile<OrgId<Self>, WeightedShareId<Self>, Self::AccountId>
    //     + LockableProfile<OrgId<Self>, WeightedShareId<Self>, Self::AccountId>
    //     + SubGroupSupervisorPermissions<OrgId<Self>, WeightedShareId<Self>, Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        OrgId = <T as Trait>::OrgId,
        <T as Trait>::ShareId,
        <T as Trait>::IpfsReference,
    {
        // ~~ FLAT, BASE ORG EVENTS ~~
        /// Organization ID, New Member Account ID
        NewMemberAddedToOrg(OrgId, AccountId),
        /// Organization ID, Old Member Account ID
        OldMemberRemovedToOrg(OrgId, AccountId),
        /// Batch Addition by the Account ID
        BatchMemberAdditionForOrg(AccountId, OrgId),
        /// Batch Removal by the Account ID
        BatchMemberRemovalForOrg(AccountId, OrgId),
        /// Organization ID, New Member Account ID
        NewMemberAddedToShareGroup(OrgId, ShareId, AccountId),
        /// Organization ID, Old Member Account ID
        OldMemberRemovedFromShareGroup(OrgId, ShareId, AccountId),
        /// Batch Addition by the Account ID
        NewMembersAddedToShareGroup(AccountId, OrgId, ShareId),
        /// Batch Removal by the Account ID
        OldMembersRemovedFromShareGroup(AccountId, OrgId, ShareId),
        // ~~ HIERARCHICAL ORG EVENTS ~~
        /// Registrar, Newly Register organization identifier, Admin ShareID, BankId, IpfsReference
        NewOrganizationRegistered(AccountId, OrgId, ShareId, IpfsReference),
        /// Registrar, OrgId, ShareId for UnWeighted Shares
        FlatInnerShareGroupAddedToOrg(AccountId, OrgId, ShareId),
        /// Registrar, OrgId, ShareId for Weighted Shares
        WeightedInnerShareGroupAddedToOrg(AccountId, OrgId, ShareId),
        /// Registrar, OrgId, ShareId for Weighted Shares
        WeightedOuterShareGroupAddedToOrg(AccountId, OrgId, ShareId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // ~~ ORG ERRORS START ~~
        UnAuthorizedSwapSudoRequest,
        NoExistingSudoKey,
        UnAuthorizedRequestToSwapSupervisor,
        NotAuthorizedToChangeMembership,
        OrganizationHasNoMembershipInStorage,
        // ~~ SHARE GROUP ERRORS START ~~
        ShareHolderGroupHasNoMembershipInStorage,
        CantBurnSharesIfReferenceCountIsNone,
        // ~~
        ShareIdTypeNotShareMembershipVariantSoCantAddMembers,
        ShareIdTypeNotAtomicSharesVariantSoCantAddMembers,
        SpinOffCannotOccurFromNonExistentShareGroup,
        /// This is an auth restriction to provide some anti-sybil mechanism for certain actions within the system
        /// - the 0th organization is registered at genesis and its members are essentially sudo
        /// - in the future, I'd like to be able to invite people to open an organization in a controlled way so that their
        /// invitation takes a unique form
        MustBeAMemberOf0thOrgToRegisterNewOrg,
        MustHaveCertainAuthorityToRegisterInnerShares,
        MustHaveCertainAuthorityToRegisterOuterShares,
        FlatShareGroupNotFound,
        WeightedShareGroupNotFound,
        NoProfileFoundForAccountToBurn,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Org {
        // ~~ ORG STORAGE ITEMS
        /// The account that can set all the organization supervisors, should be replaced by committee-based governance
        SudoKey build(|config: &GenesisConfig<T>| Some(config.omnipotent_key.clone())): Option<T::AccountId>;
        /// The account that can do a lot of supervisor only stuff for the organization
        OrganizationSupervisor get(fn organization_supervisor):
            map hasher(opaque_blake2_256) T::OrgId => Option<T::AccountId>;

        /// Identity nonce for registering organizations
        OrgIdNonce get(fn org_id_counter): T::OrgId;

        /// For registering the OrgId
        ClaimedOrganizationIdentity get(fn claimed_organization_identity) build(
            |_: &GenesisConfig<T>| { // FOR ALL GENESIS REGARDLESS OF CONFIG
                let mut zeroth_org_claimed_at_genesis = Vec::<(T::OrgId, bool)>::new();
                zeroth_org_claimed_at_genesis.push((T::OrgId::zero(), true));
                zeroth_org_claimed_at_genesis
        }): map hasher(opaque_blake2_256) T::OrgId => bool;

        /// The map to track organizational membership
        Members get(fn members): double_map
            hasher(opaque_blake2_256) T::OrgId,
            hasher(opaque_blake2_256) T::AccountId => bool;

        /// The size for each organization
        OrganizationSize get(fn organization_size): map hasher(opaque_blake2_256) T::OrgId => u32;

        // ~~ SHARE GROUP STORAGE ITEMS
        /// The account that has supervisor privileges for this share class of the organization
        /// - still less power than organization's supervisor defined in `membership`
        OrganizationShareSupervisor get(fn organization_share_supervisor): double_map
            hasher(opaque_blake2_256) T::OrgId,
            hasher(opaque_blake2_256) T::ShareId => Option<T::AccountId>;

        /// Identity nonce for registering organizations
        ShareIdCounter get(fn share_id_counter): map
            hasher(opaque_blake2_256) T::OrgId => T::ShareId;

        /// ShareIDs claimed set
        ClaimedShareIdentity get(fn claimed_share_identity): double_map
            hasher(opaque_blake2_256) T::OrgId,
            hasher(opaque_blake2_256) T::ShareId => bool;

        /// Membership reference counter to see when an AccountId should be removed from an organization
        /// - upholds invariant that member must be a member of share group to stay a member of the organization
        MembershipReferenceCounter get(fn membership_reference_counter): double_map
            hasher(opaque_blake2_256) T::OrgId,
            hasher(opaque_blake2_256) T::AccountId => u32;

        /// The map to track organizational membership by share class
        ShareHolders get(fn share_holders): double_map
            hasher(blake2_128_concat) ShareGroup<T::OrgId, T::ShareId>,
            hasher(blake2_128_concat) T::AccountId => bool;

        /// Tracks the size of each Share group
        ShareGroupSize get(fn share_group_size): double_map
            hasher(opaque_blake2_256) T::OrgId,
            hasher(opaque_blake2_256) T::ShareId => u32;
    }
    add_extra_genesis {
        // ~~ ORG GENESIS CONFIG ITEMS START ~~
        /// The sudo key for managing setup at genesis
        config(omnipotent_key): T::AccountId;
        /// All organizational memberships registered at genesis
        config(membership): Option<Vec<(T::OrgId, T::AccountId, bool)>>;
        // ~~ SHARE GENESIS CONFIG ITEMS START ~~
        config(share_supervisors): Option<Vec<(T::OrgId, T::ShareId, T::AccountId)>>;
        /// The shareholder member definition at genesis, requires consistency with other module geneses (plural of genesis)
        config(shareholder_membership): Option<Vec<(T::OrgId, T::ShareId, T::AccountId, bool)>>;
        // supervisor set to sudo according to rules of only module call in build(..)
        // config(first_organization_supervisor): T::AccountId;
        // config(first_organization_value_constitution): T::IpfsReference;
        // config(first_organization_flat_membership): Vec<T::AccountId>;

        build(|config: &GenesisConfig<T>| {
            if let Some(mem) = &config.membership {
                mem.iter().for_each(|(org_id, account, _)| {
                    <Module<T>>::add_new_member_to_org(
                        T::Origin::from(Some(config.omnipotent_key.clone()).into()),
                        *org_id,
                        account.clone(),
                    ).expect("genesis member could not be added to the organization");
                });
            }
            if let Some(sup) = &config.share_supervisors {
                sup.clone().iter().for_each(|(org, sid, acc)| {
                    OrganizationShareSupervisor::<T>::insert(org, sid, acc);
                });
            }
            // if let Some(mem) = &config.shareholder_membership {
            //     mem.iter().for_each(|(org_id, share_id, account, _)| {
            //         let share_supervisor = OrganizationShareSupervisor::<T>::get(org_id, share_id).expect("share supervisor must exist in order to add members at genesis");
            //         <Module<T>>::add_new_member(
            //             T::Origin::from(Some(share_supervisor).into()),
            //             *org_id,
            //             *share_id,
            //             account.clone(),
            //         ).expect("genesis member could not be added to the organization");
            //     });
            // }
            // <Module<T>>::register_organization_from_accounts(
            //     T::Origin::from(Some(config.first_organization_supervisor.clone()).into()),
            //     config.first_organization_value_constitution.clone(),
            //     config.first_organization_flat_membership.clone(),
            //     Some(config.first_organization_supervisor.clone())
            // ).expect("first organization config set up failed");
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Add member to organization
        #[weight = 0]
        fn add_new_member_to_org(
            origin,
            organization: T::OrgId,
            new_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::add_member_to_org(organization, new_member.clone(), false);
            Self::deposit_event(RawEvent::NewMemberAddedToOrg(organization, new_member));
            Ok(())
        }

        /// Remove member to organization
        #[weight = 0]
        fn remove_old_member_from_org(
            origin,
            organization: T::OrgId,
            old_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::remove_member_from_org(organization, old_member.clone(), false);
            Self::deposit_event(RawEvent::OldMemberRemovedToOrg(organization, old_member));
            Ok(())
        }

        // Batch add members to organization
        #[weight = 0]
        fn add_new_members_to_org(
            origin,
            organization: T::OrgId,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_add_members_to_org(organization, new_members);
            Self::deposit_event(RawEvent::BatchMemberAdditionForOrg(caller, organization));
            Ok(())
        }

        // Batch remove members from organization
        #[weight = 0]
        fn remove_old_members_from_org(
            origin,
            organization: T::OrgId,
            old_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_remove_members_from_org(organization, old_members);
            Self::deposit_event(RawEvent::BatchMemberRemovalForOrg(caller, organization));
            Ok(())
        }
        /// Add member to organization's share class
        #[weight = 0]
        fn add_new_member_to_share_group(
            origin,
            organization: T::OrgId,
            share_id: T::ShareId,
            new_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::is_sudo_key(&caller)
            //                         || Self::is_organization_supervisor(organization, &caller)
            //                         || Self::is_sub_group_supervisor(organization, share_id, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            Self::add_member_to_sub_org(organization, share_id, new_member.clone(), false);
            Self::deposit_event(RawEvent::NewMemberAddedToShareGroup(organization, share_id, new_member));
            Ok(())
        }

        /// Remove member from organization's share class
        #[weight = 0]
        fn remove_old_member_from_share_group(
            origin,
            organization: T::OrgId,
            share_id: T::ShareId,
            old_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::check_if_sudo_account(&caller)
            //                         || Self::check_if_organization_supervisor_account(organization, &caller)
            //                         || Self::is_sub_group_supervisor(organization, share_id, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            Self::remove_member_from_sub_org(organization, share_id, old_member.clone(), false);
            Self::deposit_event(RawEvent::OldMemberRemovedFromShareGroup(organization, share_id, old_member));
            Ok(())
        }

        // Batch add members to organization's share group
        #[weight = 0]
        fn add_new_members_to_share_group(
            origin,
            organization: T::OrgId,
            share_id: T::ShareId,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::check_if_sudo_account(&caller)
            //                         || Self::check_if_organization_supervisor_account(organization, &caller)
            //                         || Self::is_sub_group_supervisor(organization, share_id, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            Self::batch_add_members_to_sub_org(organization, share_id, new_members);
            Self::deposit_event(RawEvent::NewMembersAddedToShareGroup(caller, organization, share_id));
            Ok(())
        }

        // Batch remove members from organization's share class
        #[weight = 0]
        fn remove_old_members_from_share_group(
            origin,
            organization: T::OrgId,
            share_id: T::ShareId,
            old_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let authentication: bool = Self::check_if_sudo_account(&caller)
            //                         || Self::check_if_organization_supervisor_account(organization, &caller)
            //                         || Self::is_sub_group_supervisor(organization, share_id, &caller);
            // ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);
            Self::batch_remove_members_from_sub_org(organization, share_id, old_members);
            Self::deposit_event(RawEvent::OldMembersRemovedFromShareGroup(caller, organization, share_id));
            Ok(())
        }
    }
}

// ~~ ORG TRAIT IMPLS START ~~
impl<T: Trait> GetGroupSize<T::OrgId, T::ShareId> for Module<T> {
    fn get_size_of_group(org_id: T::OrgId) -> u32 {
        <OrganizationSize<T>>::get(org_id)
    }
    fn get_size_of_subgroup(org_id: T::OrgId, share_id: T::ShareId) -> u32 {
        <ShareGroupSize<T>>::get(org_id, share_id)
    }
}

impl<T: Trait> GroupMembership<T::OrgId, T::ShareId, T::AccountId> for Module<T> {
    fn is_member_of_group(org_id: T::OrgId, who: &T::AccountId) -> bool {
        <Members<T>>::get(org_id, who)
    }
    fn is_member_of_subgroup(org_id: T::OrgId, share_id: T::ShareId, who: &T::AccountId) -> bool {
        let group_id = ShareGroup::new(org_id, share_id);
        <ShareHolders<T>>::get(group_id, who)
    }
}

impl<T: Trait> IDIsAvailable<T::OrgId> for Module<T> {
    fn id_is_available(id: T::OrgId) -> bool {
        !<ClaimedOrganizationIdentity<T>>::get(id)
    }
}

// impl<T: Trait> IDIsAvailable<ShareGroup<T::OrgId, T::ShareId>> for Module<T> {
//     fn id_is_available(id: ShareGroup<T::OrgId, T::ShareId>) -> bool {
//         !<ClaimedShareIdentity<T>>::get(id.org, id.share)
//     }
// }

impl<T: Trait> GenerateUniqueID<T::OrgId> for Module<T> {
    fn generate_unique_id() -> T::OrgId {
        let mut id_counter = <OrgIdNonce<T>>::get() + 1u32.into();
        while <ClaimedOrganizationIdentity<T>>::get(id_counter) {
            // add overflow check here? not really necessary
            id_counter += 1u32.into();
        }
        <OrgIdNonce<T>>::put(id_counter);
        id_counter
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::ShareId, T::OrgId> for Module<T> {
    fn seeded_generate_unique_id(seed: T::OrgId) -> T::ShareId {
        let mut id_counter = <ShareIdCounter<T>>::get(seed) + 1u32.into();
        while <ClaimedShareIdentity<T>>::get(seed, id_counter) {
            // TODO: add overflow check here
            id_counter += 1u32.into();
        }
        <ShareIdCounter<T>>::insert(seed, id_counter);
        id_counter
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
                <SudoKey<T>>::put(new_key);
                return Ok(());
            }
            return Err(Error::<T>::UnAuthorizedSwapSudoRequest.into());
        }
        Err(Error::<T>::NoExistingSudoKey.into())
    }
}

impl<T: Trait> OrganizationSupervisorPermissions<T::OrgId, T::AccountId> for Module<T> {
    fn is_organization_supervisor(org: T::OrgId, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::organization_supervisor(org) {
            return who == &supervisor;
        }
        false
    }
    // set the supervisor for the organization in whatever context
    fn put_organization_supervisor(org: T::OrgId, who: T::AccountId) {
        <OrganizationSupervisor<T>>::insert(org, who);
    }
    // sudo key and the current supervisor have the power to change the supervisor
    fn set_organization_supervisor(
        org: T::OrgId,
        old_supervisor: &T::AccountId,
        new_supervisor: T::AccountId,
    ) -> DispatchResult {
        let authentication: bool = Self::is_organization_supervisor(org, &old_supervisor)
            || Self::is_sudo_key(&old_supervisor);
        if authentication {
            <OrganizationSupervisor<T>>::insert(org, new_supervisor);
            return Ok(());
        }
        Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
    }
}

impl<T: Trait> ChangeGroupMembership<T::OrgId, T::ShareId, T::AccountId> for Module<T> {
    fn add_member_to_org(org_id: T::OrgId, new_member: T::AccountId, batch: bool) {
        if !batch {
            if !<ClaimedOrganizationIdentity<T>>::get(org_id) {
                <ClaimedOrganizationIdentity<T>>::insert(org_id, true);
            }
            let new_organization_size = <OrganizationSize<T>>::get(org_id) + 1u32;
            <OrganizationSize<T>>::insert(org_id, new_organization_size);
        }
        <Members<T>>::insert(org_id, new_member, true);
    }
    fn remove_member_from_org(org_id: T::OrgId, old_member: T::AccountId, batch: bool) {
        if !batch {
            let new_organization_size = <OrganizationSize<T>>::get(org_id).saturating_sub(1u32);
            <OrganizationSize<T>>::insert(org_id, new_organization_size);
        }
        <Members<T>>::insert(org_id, old_member, false);
    }
    fn add_member_to_sub_org(
        org_id: T::OrgId,
        share_id: T::ShareId,
        new_member: T::AccountId,
        batch: bool,
    ) {
        if !<ClaimedShareIdentity<T>>::get(org_id, share_id) {
            <ClaimedShareIdentity<T>>::insert(org_id, share_id, true);
        }
        if !Self::is_member_of_group(org_id, &new_member) {
            Self::add_member_to_org(org_id, new_member.clone(), false);
        }
        if !batch {
            let new_share_group_size: u32 = <ShareGroupSize<T>>::get(org_id, share_id) + 1u32;
            <ShareGroupSize<T>>::insert(org_id, share_id, new_share_group_size);
        }
        let group_id = ShareGroup::new(org_id, share_id);
        <ShareHolders<T>>::insert(group_id, &new_member, true);
        let new_membership_rc = <MembershipReferenceCounter<T>>::get(org_id, &new_member) + 1u32;
        <MembershipReferenceCounter<T>>::insert(org_id, new_member, new_membership_rc);
    }
    fn remove_member_from_sub_org(
        org_id: T::OrgId,
        share_id: T::ShareId,
        old_member: T::AccountId,
        batch: bool,
    ) {
        if !batch {
            let new_share_group_size: u32 =
                <ShareGroupSize<T>>::get(org_id, share_id).saturating_sub(1u32);
            <ShareGroupSize<T>>::insert(org_id, share_id, new_share_group_size);
        }
        let membership_rc =
            <MembershipReferenceCounter<T>>::get(org_id, &old_member).saturating_sub(1u32);
        let group_id = ShareGroup::new(org_id, share_id);
        // remove from share group
        <ShareHolders<T>>::insert(group_id, &old_member, false);
        // if rc is 0, remove from organization
        if membership_rc == 0 {
            Self::remove_member_from_org(org_id, old_member.clone(), false);
        }
        <MembershipReferenceCounter<T>>::insert(org_id, &old_member, membership_rc);
    }
    /// WARNING: the vector fed as inputs to the following methods must have NO duplicates
    fn batch_add_members_to_org(org_id: T::OrgId, new_members: Vec<T::AccountId>) {
        let size_increase: u32 = new_members.len() as u32;
        // TODO: make this a saturating add to prevent overflow attack
        let new_organization_size: u32 =
            <OrganizationSize<T>>::get(org_id).saturating_add(size_increase);
        <OrganizationSize<T>>::insert(org_id, new_organization_size);
        new_members.into_iter().for_each(|member| {
            Self::add_member_to_org(org_id, member, true);
        });
    }
    fn batch_remove_members_from_org(org_id: T::OrgId, old_members: Vec<T::AccountId>) {
        let size_decrease: u32 = old_members.len() as u32;
        let new_organization_size: u32 =
            <OrganizationSize<T>>::get(org_id).saturating_sub(size_decrease);
        <OrganizationSize<T>>::insert(org_id, new_organization_size);
        old_members.into_iter().for_each(|member| {
            Self::remove_member_from_org(org_id, member, true);
        });
    }
    fn batch_add_members_to_sub_org(
        org_id: T::OrgId,
        share_id: T::ShareId,
        new_members: Vec<T::AccountId>,
    ) {
        let size_increase = new_members.len() as u32;
        let new_share_group_size: u32 = <ShareGroupSize<T>>::get(org_id, share_id) + size_increase;
        new_members.into_iter().for_each(|member| {
            Self::add_member_to_sub_org(org_id, share_id, member, true);
        });
        <ShareGroupSize<T>>::insert(org_id, share_id, new_share_group_size);
    }
    fn batch_remove_members_from_sub_org(
        org_id: T::OrgId,
        share_id: T::ShareId,
        old_members: Vec<T::AccountId>,
    ) {
        let size_decrease = old_members.len() as u32;
        let new_share_group_size: u32 =
            <ShareGroupSize<T>>::get(org_id, share_id).saturating_sub(size_decrease);
        old_members.into_iter().for_each(|member| {
            Self::remove_member_from_sub_org(org_id, share_id, member, true);
        });
        <ShareGroupSize<T>>::insert(org_id, share_id, new_share_group_size);
    }
}

// impl<T: Trait> GetFlatShareGroup<T::OrgId, T::ShareId, T::AccountId> for Module<T> {
//     fn get_organization_share_group(
//         organization: T::OrgId,
//         share_id: T::ShareId,
//     ) -> Option<Vec<T::AccountId>> {
//         let input_share_group = ShareGroup::new(organization, share_id);
//         if !Self::id_is_available(input_share_group) {
//             Some(
//                 // is this the same performance as a get() with Key=Vec<AccountId>
//                 <ShareHolders<T>>::iter()
//                     .filter(|(share_group, _, _)| share_group == &input_share_group)
//                     .map(|(_, account, _)| account)
//                     .collect::<Vec<_>>(),
//             )
//         } else {
//             None
//         }
//     }
// }

// impl<T: Trait> SubGroupSupervisorPermissions<T::OrgId, T::ShareId, T::AccountId> for Module<T> {
//     fn is_sub_group_supervisor(org: T::OrgId, sub_group: T::ShareId, who: &T::AccountId) -> bool {
//         if let Some(supervisor) = Self::organization_share_supervisor(org, sub_group) {
//             return who == &supervisor;
//         }
//         false
//     }
//     fn put_sub_group_supervisor(org: T::OrgId, sub_group: T::ShareId, who: T::AccountId) {
//         <OrganizationShareSupervisor<T>>::insert(org, sub_group, who)
//     }
//     fn set_sub_group_supervisor(
//         org: T::OrgId,
//         sub_group: T::ShareId,
//         old_supervisor: &T::AccountId,
//         new_supervisor: T::AccountId,
//     ) -> DispatchResult {
//         let authentication: bool = Self::is_sudo_account(old_supervisor)
//             || Self::is_organization_supervisor_account(org, old_supervisor)
//             || Self::is_sub_group_supervisor(org, sub_group, old_supervisor);
//         if authentication {
//             <OrganizationShareSupervisor<T>>::insert(org, sub_group, new_supervisor);
//             return Ok(());
//         }
//         Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
//     }
// }

// ~~ ORG TRAIT IMPLS START ~~

// impl<T: Trait> OrgChecks<u32, T::AccountId> for Module<T> {
//     fn check_org_existence(org: u32) -> bool {
//         !<<T as Trait>::OrgData as IDIsAvailable<u32>>::id_is_available(org)
//     }
//     fn check_membership_in_org(org: u32, account: &T::AccountId) -> bool {
//         <<T as Trait>::OrgData as GroupMembership<
//                 <T as frame_system::Trait>::AccountId,
//         >>::is_member_of_group(
//             org, account
//         )
//     }
//     fn get_org_size(org: u32) -> u32 {
//         <<T as Trait>::OrgData as GetGroupSize>::get_size_of_group(org)
//     }
// }

// impl<T: Trait> ShareGroupChecks<T::OrgId, ShareID<FlatShareId<T>, WeightedShareId<T>>, T::AccountId>
//     for Module<T>
// {
//     fn check_share_group_existence(
//         org_id: T::OrgId,
//         share_group: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//     ) -> bool {
//         match share_group {
//             ShareID::Flat(sid) => {
//                 let prefix = ShareGroup::new(org_id, sid);
//                 !<<T as Trait>::FlatShareData as IDIsAvailable<ShareGroup>>::id_is_available(prefix)
//             }
//             ShareID::Weighted(sid) => {
//                 let prefix = ShareGroup::new(org_id, sid);
//                 !<<T as Trait>::WeightedShareData as IDIsAvailable<ShareGroup>>::id_is_available(
//                     prefix,
//                 )
//             }
//         }
//     }
//     fn check_membership_in_share_group(
//         org_id: T::OrgId,
//         share_group: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//         account: &T::AccountId,
//     ) -> bool {
//         match share_group {
//             ShareID::Flat(share_id) => {
//                 let group_id = ShareGroup::new(org_id, share_id);
//                 <<T as Trait>::FlatShareData as GroupMembership<
//                     <T as frame_system::Trait>::AccountId,
//                 >>::is_member_of_group(group_id, account)
//             }
//             ShareID::Weighted(share_id) => {
//                 let group_id = ShareGroup::new(org_id, share_id);
//                 <<T as Trait>::WeightedShareData as GroupMembership<
//                     <T as frame_system::Trait>::AccountId,
//                 >>::is_member_of_group(group_id, account)
//             }
//         }
//     }
//     fn get_share_group_size(
//         org: T::OrgId,
//         share_group: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//     ) -> u32 {
//         match share_group {
//             ShareID::Flat(share_id) => {
//                 let group_id = ShareGroup::new(org, share_id);
//                 <<T as Trait>::FlatShareData as GetGroupSize>::get_size_of_group(group_id)
//             }
//             ShareID::Weighted(share_id) => {
//                 let group_id = ShareGroup::new(org, share_id);
//                 <<T as Trait>::WeightedShareData as GetGroupSize>::get_size_of_group(group_id)
//             }
//         }
//     }
// }

// impl<T: Trait>
//     SupervisorPermissions<T::OrgId, ShareID<FlatShareId<T>, WeightedShareId<T>>, T::AccountId>
//     for Module<T>
// {
//     fn is_sudo_account(who: &T::AccountId) -> bool {
//         <<T as Trait>::OrgData as ChainSudoPermissions<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
//     }
//     fn is_organization_supervisor(organization: T::OrgId, who: &T::AccountId) -> bool {
//         <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
//             T::OrgId,
//             <T as frame_system::Trait>::AccountId,
//         >>::is_organization_supervisor(organization, who)
//     }
//     fn is_share_supervisor(
//         organization: T::OrgId,
//         share_id: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//         who: &T::AccountId,
//     ) -> bool {
//         match share_id {
//             ShareID::Flat(sid) => <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
//                 T::OrgId,
//                 FlatShareId<T>,
//                 <T as frame_system::Trait>::AccountId,
//             >>::is_sub_group_supervisor(organization, sid, who),
//             ShareID::Weighted(sid) => {
//                 <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
//                     T::OrgId,
//                     WeightedShareId<T>,
//                     <T as frame_system::Trait>::AccountId,
//                 >>::is_sub_group_supervisor(organization, sid, who)
//             }
//         }
//     }
//     // infallible, not protected in any way
//     fn put_sudo_account(who: T::AccountId) {
//         <<T as Trait>::OrgData as ChainSudoPermissions<
//             <T as frame_system::Trait>::AccountId,
//         >>::put_sudo_key(who);
//     }
//     fn put_organization_supervisor(organization: T::OrgId, who: T::AccountId) {
//         <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
//             T::OrgId,
//             <T as frame_system::Trait>::AccountId,
//         >>::put_organization_supervisor(organization, who);
//     }
//     fn put_share_group_supervisor(
//         organization: T::OrgId,
//         share_id: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//         who: T::AccountId,
//     ) {
//         match share_id {
//             ShareID::Flat(sid) => {
//                 <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
//                     T::OrgId,
//                     FlatShareId<T>,
//                     <T as frame_system::Trait>::AccountId,
//                 >>::put_sub_group_supervisor(organization, sid, who);
//             }
//             ShareID::Weighted(sid) => {
//                 <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
//                     T::OrgId,
//                     WeightedShareId<T>,
//                     <T as frame_system::Trait>::AccountId,
//                 >>::put_sub_group_supervisor(organization, sid, who);
//             }
//         }
//     }
//     // CAS by default to enforce existing permissions and isolate logic
//     fn set_sudo_account(setter: &T::AccountId, new: T::AccountId) -> DispatchResult {
//         <<T as Trait>::OrgData as ChainSudoPermissions<
//             <T as frame_system::Trait>::AccountId,
//         >>::set_sudo_key(setter, new)
//     }
//     fn set_organization_supervisor(
//         organization: T::OrgId,
//         setter: &T::AccountId,
//         new: T::AccountId,
//     ) -> DispatchResult {
//         <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
//             T::OrgId,
//             <T as frame_system::Trait>::AccountId,
//         >>::set_organization_supervisor(organization, setter, new)
//     }
//     fn set_share_supervisor(
//         organization: T::OrgId,
//         share_id: ShareID<FlatShareId<T>, WeightedShareId<T>>,
//         setter: &T::AccountId,
//         new: T::AccountId,
//     ) -> DispatchResult {
//         match share_id {
//             ShareID::Flat(sid) => {
//                 <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
//                     T::OrgId,
//                     FlatShareId<T>,
//                     <T as frame_system::Trait>::AccountId,
//                 >>::set_sub_group_supervisor(organization, sid, setter, new)
//             }
//             ShareID::Weighted(sid) => {
//                 <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
//                     T::OrgId,
//                     WeightedShareId<T>,
//                     <T as frame_system::Trait>::AccountId,
//                 >>::set_sub_group_supervisor(organization, sid, setter, new)
//             }
//         }
//     }
// }

// impl<T: Trait> FlatShareWrapper<T::OrgId, FlatShareId<T>, T::AccountId> for Module<T> {
//     fn get_flat_share_group(
//         organization: T::OrgId,
//         share_id: FlatShareId<T>,
//     ) -> Result<Vec<T::AccountId>, DispatchError> {
//         let ret = <<T as Trait>::FlatShareData as GetFlatShareGroup<
//             T::OrgId,
//             FlatShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::get_organization_share_group(organization, share_id)
//         .ok_or(Error::<T>::FlatShareGroupNotFound)?;
//         Ok(ret)
//     }
//     fn generate_unique_flat_share_id(organization: u32) -> u32 {
//         <<T as Trait>::FlatShareData as SeededGenerateUniqueID<FlatShareId<T>, T::OrgId>>::seeded_generate_unique_id(
//             organization,
//         )
//     }
//     fn add_members_to_flat_share_group(
//         organization: T::OrgId,
//         share_id: FlatShareId<T>,
//         members: Vec<T::AccountId>,
//     ) {
//         let prefix = ShareGroup::new(organization, share_id);
//         <<T as Trait>::FlatShareData as ChangeGroupMembership<
//             <T as frame_system::Trait>::AccountId,
//         >>::batch_add_group_members(prefix, members);
//     }
// }

// impl<T: Trait> WeightedShareWrapper<T::OrgId, WeightedShareId<T>, T::AccountId> for Module<T> {
//     type Shares = SharesOf<T>; // exists only to pass inheritance to modules that inherit org
//     type Profile = <<T as Trait>::WeightedShareData as WeightedShareGroup<
//         T::OrgId,
//         WeightedShareId<T>,
//         <T as frame_system::Trait>::AccountId,
//     >>::Profile;
//     type Genesis = SimpleShareGenesis<T::AccountId, Self::Shares>;
//     fn get_member_share_profile(
//         organization: T::OrgId,
//         share_id: WeightedShareId<T>,
//         member: &T::AccountId,
//     ) -> Option<Self::Profile> {
//         <<T as Trait>::WeightedShareData as WeightedShareGroup<
//             T::OrgId,
//             WeightedShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::get_share_profile(organization, share_id, member)
//     }
//     fn get_weighted_share_group(
//         organization: T::OrgId,
//         share_id: WeightedShareId<T>,
//     ) -> Result<Self::Genesis, DispatchError> {
//         let ret = <<T as Trait>::WeightedShareData as WeightedShareGroup<
//             T::OrgId,
//             WeightedShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::shareholder_membership(organization, share_id)
//         .ok_or(Error::<T>::WeightedShareGroupNotFound)?;
//         Ok(ret.into())
//     }
//     fn get_outstanding_weighted_shares(
//         organization: T::OrgId,
//         share_id: WeightedShareId<T>,
//     ) -> Option<Self::Shares> {
//         <<T as Trait>::WeightedShareData as WeightedShareGroup<
//             T::OrgId,
//             WeightedShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::outstanding_shares(organization, share_id)
//     }
//     fn generate_unique_weighted_share_id(organization: T::OrgId) -> WeightedShareId<T> {
//         <<T as Trait>::WeightedShareData as SeededGenerateUniqueID<WeightedShareId<T>, T::OrgId>>::seeded_generate_unique_id(
//             organization,
//         )
//     }
// }

// impl<T: Trait> WeightedShareIssuanceWrapper<T::OrgId, WeightedShareId<T>, T::AccountId, Permill>
//     for Module<T>
// {
//     fn issue_weighted_shares_from_accounts(
//         organization: T::OrgId,
//         members: Vec<(T::AccountId, Self::Shares)>,
//     ) -> Result<u32, DispatchError> {
//         let share_id = Self::generate_unique_weighted_share_id(organization);
//         <<T as Trait>::WeightedShareData as ShareBank<
//             T::OrgId,
//             WeightedShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::batch_issue(organization, share_id, members.into())?;
//         Ok(share_id)
//     }
//     fn burn_weighted_shares_for_member(
//         organization: T::OrgId,
//         share_id: WeightedShareId<T>,
//         account: T::AccountId,
//         // TODO: make portion an enum that expresses amount or permill
//         // execute logic in here to decide that amount
//         amount_to_burn: Option<Permill>,
//     ) -> Result<Self::Shares, DispatchError> {
//         let total_shares = Self::get_member_share_profile(organization, share_id, &account)
//             .ok_or(Error::<T>::NoProfileFoundForAccountToBurn)?
//             .total();
//         let shares_to_burn = if let Some(pct_2_burn) = amount_to_burn {
//             pct_2_burn * total_shares
//         } else {
//             total_shares
//         };
//         <<T as Trait>::WeightedShareData as ShareBank<
//             T::OrgId,
//             WeightedShareId<T>,
//             <T as frame_system::Trait>::AccountId,
//         >>::burn(
//             organization,
//             share_id,
//             account,
//             shares_to_burn.clone(),
//             false,
//         )?;
//         Ok(shares_to_burn)
//     }
// }

// impl<T: Trait>
//     RegisterShareGroup<
//         T::OrgId,
//         ShareID<FlatShareId<T>, WeightedShareId<T>>,
//         T::AccountId,
//         SharesOf<T>,
//     > for Module<T>
// {
//     fn register_inner_flat_share_group(
//         organization: T::OrgId,
//         group: Vec<T::AccountId>,
//     ) -> Result<ShareID<FlatShareId<T>, WeightedShareId<T>>, DispatchError> {
//         let raw_share_id = Self::generate_unique_flat_share_id(organization);
//         Self::add_members_to_flat_share_group(organization, raw_share_id, group);
//         let new_share_id = ShareID::Flat(raw_share_id);
//         OrganizationInnerShares::insert(organization, new_share_id, true);
//         Ok(new_share_id)
//     }
//     fn register_inner_weighted_share_group(
//         organization: T::OrgId,
//         group: Vec<(T::AccountId, SharesOf<T>)>,
//     ) -> Result<ShareID<FlatShareId<T>, WeightedShareId<T>>, DispatchError> {
//         let raw_share_id = Self::issue_weighted_shares_from_accounts(organization, group)?;
//         let new_share_id = ShareID::Weighted(raw_share_id);
//         OrganizationInnerShares::insert(organization, new_share_id, true);
//         Ok(new_share_id)
//     }
//     fn register_outer_flat_share_group(
//         organization: T::OrgId,
//         group: Vec<T::AccountId>,
//     ) -> Result<ShareID<FlatShareId<T>, WeightedShareId<T>>, DispatchError> {
//         let raw_share_id = Self::generate_unique_flat_share_id(organization);
//         Self::add_members_to_flat_share_group(organization, raw_share_id, group);
//         let new_share_id = ShareID::Flat(raw_share_id);
//         OrganizationOuterShares::insert(organization, new_share_id, true);
//         Ok(new_share_id)
//     }
//     fn register_outer_weighted_share_group(
//         organization: T::OrgId,
//         group: Vec<(T::AccountId, SharesOf<T>)>,
//     ) -> Result<ShareID<FlatShareId<T>, WeightedShareId<T>>, DispatchError> {
//         let raw_share_id = Self::issue_weighted_shares_from_accounts(organization, group)?;
//         let new_share_id = ShareID::Weighted(raw_share_id);
//         OrganizationOuterShares::insert(organization, new_share_id, true);
//         Ok(new_share_id)
//     }
// }

// impl<T: Trait>
//     GetInnerOuterShareGroups<T::OrgId, ShareID<FlatShareId<T>, WeightedShareId<T>>, T::AccountId>
//     for Module<T>
// {
//     fn get_inner_share_group_identifiers(
//         organization: T::OrgId,
//     ) -> Option<Vec<ShareID<FlatShareId<T>, WeightedShareId<T>>>> {
//         let inner_shares = <OrganizationInnerShares<T>>::iter()
//             .filter(|(org, _, exists)| (org == &organization) && *exists)
//             .map(|(_, share_id, _)| share_id)
//             .collect::<Vec<_>>();
//         if inner_shares.is_empty() {
//             None
//         } else {
//             Some(inner_shares)
//         }
//     }
//     fn get_outer_share_group_identifiers(
//         organization: T::OrgId,
//     ) -> Option<Vec<ShareID<FlatShareId<T>, WeightedShareId<T>>>> {
//         let outer_shares = <OrganizationOuterShares<T>>::iter()
//             .filter(|(org, _, exists)| (org == &organization) && *exists)
//             .map(|(_, share_id, _)| share_id)
//             .collect::<Vec<_>>();
//         if outer_shares.is_empty() {
//             None
//         } else {
//             Some(outer_shares)
//         }
//     }
// }

// impl<T: Trait> OrganizationDNS<T::OrgId, T::AccountId, T::IpfsReference> for Module<T> {
//     type OrgSrc = OrganizationSource<FlatShareId<T>, WeightedShareId<T>, T::AccountId, SharesOf<T>>;
//     type OrganizationState = Organization<FlatShareId<T>, WeightedShareId<T>, T::IpfsReference>;
//     /// This method registers all the necessary ShareId and Power Structures from `Self::OrgSrc` and returns the `OrganizationState`
//     /// which contains all the identifiers to track all of those power structures independently within the inherited modules
//     fn organization_from_src(
//         src: Self::OrgSrc,
//         organization: T::OrgId,
//         value_constitution: T::IpfsReference,
//     ) -> Result<Self::OrganizationState, DispatchError> {
//         let share_id = match src {
//             OrganizationSource::Accounts(unweighted_members) => {
//                 // register shares membership group with this membership set
//                 let new_share_id = Self::generate_unique_flat_share_id(organization);
//                 Self::add_members_to_flat_share_group(
//                     organization,
//                     new_share_id,
//                     unweighted_members,
//                 );
//                 ShareID::Flat(new_share_id)
//             }
//             OrganizationSource::AccountsWeighted(weighted_members) => {
//                 let raw_share_id =
//                     Self::issue_weighted_shares_from_accounts(organization, weighted_members)?;
//                 ShareID::Weighted(raw_share_id)
//             }
//             OrganizationSource::SpinOffShareGroup(org_id, share_id) => {
//                 // check existence of the share group with this identifier and return the variant
//                 let existence_check = Self::check_share_group_existence(org_id, share_id);
//                 ensure!(
//                     existence_check,
//                     Error::<T>::SpinOffCannotOccurFromNonExistentShareGroup
//                 );
//                 match share_id {
//                     ShareID::Flat(sid) => {
//                         // register it as a new organization and set it as the set
//                         let new_members = Self::get_flat_share_group(org_id, sid)?;
//                         let new_share_id = Self::generate_unique_flat_share_id(organization);
//                         Self::add_members_to_flat_share_group(
//                             organization,
//                             new_share_id,
//                             new_members,
//                         );
//                         ShareID::Flat(new_share_id)
//                     }
//                     ShareID::Weighted(sid) => {
//                         let new_weighted_membership =
//                             Self::get_weighted_share_group(org_id, sid)?.account_ownership();
//                         let raw_share_id = Self::issue_weighted_shares_from_accounts(
//                             organization,
//                             new_weighted_membership,
//                         )?;
//                         ShareID::Weighted(raw_share_id)
//                     }
//                 }
//             }
//         };
//         // TODO: symmetric API for deletion (removal and state gc)
//         <OrganizationInnerShares<T>>::insert(organization, share_id, true);
//         // use it to create an organization
//         let new_organization = Organization::new(share_id, value_constitution);
//         Ok(new_organization)
//     }
//     fn register_organization(
//         source: Self::OrgSrc,
//         value_constitution: T::IpfsReference,
//         supervisor: Option<T::AccountId>,
//     ) -> Result<(u32, Self::OrganizationState), DispatchError> {
//         let new_org_id =
//             <<T as Trait>::OrgData as GenerateUniqueID<T::OrgId>>::generate_unique_id();
//         // use helper method to register everything and return main storage item for tracking associated state/permissions for org
//         let new_organization_state =
//             Self::organization_from_src(source, new_org_id, value_constitution)?;
//         // insert _canonical_ state of the organization
//         OrganizationStates::insert(new_org_id, new_organization_state.clone());
//         // assign the supervisor to supervisor if that assignment is specified
//         if let Some(org_sudo) = supervisor {
//             Self::put_organization_supervisor(new_org_id, org_sudo);
//         }
//         // iterate the OrganizationCounter
//         let new_counter = OrganizationCounter::get() + 1u32;
//         OrganizationCounter::put(new_counter);
//         Ok((new_org_id, new_organization_state))
//     }
// }
