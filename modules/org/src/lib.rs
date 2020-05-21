#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use util::{
    organization::{Organization, OrganizationSource, ShareID},
    share::SimpleShareGenesis,
    traits::{
        AccessGenesis, AccessProfile, ChainSudoPermissions, ChangeGroupMembership,
        FlatShareWrapper, GenerateUniqueID, GetFlatShareGroup, GetGroupSize,
        GetInnerOuterShareGroups, GroupMembership, IDIsAvailable, LockableProfile, OrgChecks,
        OrganizationDNS, OrganizationSupervisorPermissions, RegisterShareGroup, ReservableProfile,
        SeededGenerateUniqueID, ShareBank, ShareGroupChecks, SubGroupSupervisorPermissions,
        SupervisorPermissions, WeightedShareGroup, WeightedShareIssuanceWrapper,
        WeightedShareWrapper,
    },
    uuid::UUID2,
};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult, Permill};
use sp_std::prelude::*;

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::WeightedShareData as WeightedShareGroup<
    <T as frame_system::Trait>::AccountId,
>>::Shares;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Used for permissions and shared for organizational membership in both
    /// shares modules
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<u32>
        + GenerateUniqueID<u32>
        + ChainSudoPermissions<Self::AccountId>
        + OrganizationSupervisorPermissions<u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>;

    /// Used for shares-membership -> vote-petition
    type FlatShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId, GroupId = UUID2>
        + IDIsAvailable<UUID2>
        + SeededGenerateUniqueID<u32, u32>
        + SubGroupSupervisorPermissions<u32, u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>
        + GetFlatShareGroup<Self::AccountId>;

    /// Used for shares-atomic -> vote-yesno
    /// - this is NOT synced with FlatShareData
    /// so the `SharesOf<T>` and `ShareId` checks must be treated separately
    type WeightedShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<UUID2>
        + SeededGenerateUniqueID<u32, u32>
        + WeightedShareGroup<Self::AccountId>
        + ShareBank<Self::AccountId>
        + ReservableProfile<Self::AccountId>
        + LockableProfile<Self::AccountId>
        + SubGroupSupervisorPermissions<u32, u32, Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        /// Registrar, Newly Register organization identifier, Admin ShareID, BankId, IpfsReference
        NewOrganizationRegistered(AccountId, u32, ShareID, IpfsReference),
        /// Registrar, OrgId, ShareId for UnWeighted Shares
        FlatInnerShareGroupAddedToOrg(AccountId, u32, ShareID),
        /// Registrar, OrgId, ShareId for Weighted Shares
        WeightedInnerShareGroupAddedToOrg(AccountId, u32, ShareID),
        /// Registrar, OrgId, ShareId for Weighted Shares
        WeightedOuterShareGroupAddedToOrg(AccountId, u32, ShareID),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
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
        /// The number of organizations in this module (eventually add nuance like `Active` tiers)
        OrganizationCounter get(fn organization_counter): u32;

        /// Organizations that were registered in this module
        OrganizationStates get(fn organization_states): map
            hasher(opaque_blake2_256) u32 => Option<Organization<IpfsReference>>;

        /// Organization Inner Shares
        /// - purpose: organizational governance, task supervision
        OrganizationInnerShares get(fn organization_inner_shares): double_map
            hasher(blake2_128_concat) u32,
            hasher(blake2_128_concat) ShareID => bool;

        /// Organization Outer Shares
        /// - purpose: funded teams ownership enforcement, external contractors
        OrganizationOuterShares get(fn organization_outer_shares): double_map
            hasher(blake2_128_concat) u32,
            hasher(blake2_128_concat) ShareID => bool;
    }
    add_extra_genesis {
        // supervisor set to sudo according to rules of only module call in build(..)
        config(first_organization_supervisor): T::AccountId;
        config(first_organization_value_constitution): IpfsReference;
        config(first_organization_flat_membership): Vec<T::AccountId>;

        build(|config: &GenesisConfig<T>| {
            <Module<T>>::register_organization_from_accounts(
                T::Origin::from(Some(config.first_organization_supervisor.clone()).into()),
                config.first_organization_value_constitution.clone(),
                config.first_organization_flat_membership.clone(),
                Some(config.first_organization_supervisor.clone())
            ).expect("first organization config set up failed");
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_organization_from_accounts(
            origin,
            value_constitution: IpfsReference,
            accounts: Vec<T::AccountId>,
            supervisor: Option<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&caller)
                || Self::check_membership_in_org(1u32, &caller);
            ensure!(authentication, Error::<T>::MustBeAMemberOf0thOrgToRegisterNewOrg);

            let new_organization_state = Self::register_organization(OrganizationSource::<_, SharesOf<T>>::Accounts(accounts), value_constitution, supervisor)?;
            Self::deposit_event(RawEvent::NewOrganizationRegistered(caller, new_organization_state.0, new_organization_state.1.admin_id(), new_organization_state.1.constitution()));
            Ok(())
        }
        #[weight = 0]
        fn register_inner_flat_share_group_for_organization(
            origin,
            organization: u32,
            group: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&caller)
                || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterInnerShares);

            let new_share_id = Self::register_inner_flat_share_group(organization, group)?;
            Self::deposit_event(RawEvent::FlatInnerShareGroupAddedToOrg(caller, organization, new_share_id));
            Ok(())
        }
        #[weight = 0]
        fn register_inner_weighted_share_group_for_organization(
            origin,
            organization: u32,
            group: Vec<(T::AccountId, SharesOf<T>)>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&caller)
                || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterInnerShares);

            let new_share_id = Self::register_inner_weighted_share_group(organization, group)?;
            Self::deposit_event(RawEvent::WeightedInnerShareGroupAddedToOrg(caller, organization, new_share_id));
            Ok(())
        }
        #[weight = 0]
        fn register_outer_weighted_share_group_for_organization(
            origin,
            organization: u32,
            group: Vec<(T::AccountId, SharesOf<T>)>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&caller)
                || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterOuterShares);

            let new_share_id = Self::register_outer_weighted_share_group(organization, group)?;
            Self::deposit_event(RawEvent::WeightedOuterShareGroupAddedToOrg(caller, organization, new_share_id));
            Ok(())
        }
        // setting all permissions
    }
}

impl<T: Trait> OrgChecks<u32, T::AccountId> for Module<T> {
    fn check_org_existence(org: u32) -> bool {
        !<<T as Trait>::OrgData as IDIsAvailable<u32>>::id_is_available(org)
    }
    fn check_membership_in_org(org: u32, account: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as GroupMembership<
                <T as frame_system::Trait>::AccountId,
        >>::is_member_of_group(
            org, account
        )
    }
    fn get_org_size(org: u32) -> u32 {
        <<T as Trait>::OrgData as GetGroupSize>::get_size_of_group(org)
    }
}

impl<T: Trait> ShareGroupChecks<u32, T::AccountId> for Module<T> {
    type MultiShareIdentifier = ShareID;
    fn check_share_group_existence(org_id: u32, share_group: Self::MultiShareIdentifier) -> bool {
        match share_group {
            ShareID::Flat(sid) => {
                let prefix = UUID2::new(org_id, sid);
                !<<T as Trait>::FlatShareData as IDIsAvailable<UUID2>>::id_is_available(prefix)
            }
            ShareID::WeightedAtomic(sid) => {
                let prefix = UUID2::new(org_id, sid);
                !<<T as Trait>::WeightedShareData as IDIsAvailable<UUID2>>::id_is_available(prefix)
            }
        }
    }
    fn check_membership_in_share_group(
        org_id: u32,
        share_group: Self::MultiShareIdentifier,
        account: &T::AccountId,
    ) -> bool {
        match share_group {
            ShareID::Flat(share_id) => {
                let group_id = UUID2::new(org_id, share_id);
                <<T as Trait>::FlatShareData as GroupMembership<
                    <T as frame_system::Trait>::AccountId,
                >>::is_member_of_group(group_id, account)
            }
            ShareID::WeightedAtomic(share_id) => {
                let group_id = UUID2::new(org_id, share_id);
                <<T as Trait>::WeightedShareData as GroupMembership<
                    <T as frame_system::Trait>::AccountId,
                >>::is_member_of_group(group_id, account)
            }
        }
    }
    fn get_share_group_size(org: u32, share_group: Self::MultiShareIdentifier) -> u32 {
        match share_group {
            ShareID::Flat(share_id) => {
                let group_id = UUID2::new(org, share_id);
                <<T as Trait>::FlatShareData as GetGroupSize>::get_size_of_group(group_id)
            }
            ShareID::WeightedAtomic(share_id) => {
                let group_id = UUID2::new(org, share_id);
                <<T as Trait>::WeightedShareData as GetGroupSize>::get_size_of_group(group_id)
            }
        }
    }
}

impl<T: Trait> SupervisorPermissions<u32, T::AccountId> for Module<T> {
    fn is_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as ChainSudoPermissions<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
    }
    fn is_organization_supervisor(organization: u32, who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::is_organization_supervisor(organization, who)
    }
    fn is_share_supervisor(
        organization: u32,
        share_id: Self::MultiShareIdentifier,
        who: &T::AccountId,
    ) -> bool {
        match share_id {
            ShareID::Flat(sid) => <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
                u32,
                u32,
                <T as frame_system::Trait>::AccountId,
            >>::is_sub_group_supervisor(organization, sid, who),
            ShareID::WeightedAtomic(sid) => {
                <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
                    u32,
                    u32,
                    <T as frame_system::Trait>::AccountId,
                >>::is_sub_group_supervisor(organization, sid, who)
            }
        }
    }
    // infallible, not protected in any way
    fn put_sudo_account(who: T::AccountId) {
        <<T as Trait>::OrgData as ChainSudoPermissions<
            <T as frame_system::Trait>::AccountId,
        >>::put_sudo_key(who);
    }
    fn put_organization_supervisor(organization: u32, who: T::AccountId) {
        <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::put_organization_supervisor(organization, who);
    }
    fn put_share_group_supervisor(
        organization: u32,
        share_id: Self::MultiShareIdentifier,
        who: T::AccountId,
    ) {
        match share_id {
            ShareID::Flat(sid) => {
                <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
                    u32,
                    u32,
                    <T as frame_system::Trait>::AccountId,
                >>::put_sub_group_supervisor(organization, sid, who);
            }
            ShareID::WeightedAtomic(sid) => {
                <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
                    u32,
                    u32,
                    <T as frame_system::Trait>::AccountId,
                >>::put_sub_group_supervisor(organization, sid, who);
            }
        }
    }
    // CAS by default to enforce existing permissions and isolate logic
    fn set_sudo_account(setter: &T::AccountId, new: T::AccountId) -> DispatchResult {
        <<T as Trait>::OrgData as ChainSudoPermissions<
            <T as frame_system::Trait>::AccountId,
        >>::set_sudo_key(setter, new)
    }
    fn set_organization_supervisor(
        organization: u32,
        setter: &T::AccountId,
        new: T::AccountId,
    ) -> DispatchResult {
        <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::set_organization_supervisor(organization, setter, new)
    }
    fn set_share_supervisor(
        organization: u32,
        share_id: Self::MultiShareIdentifier,
        setter: &T::AccountId,
        new: T::AccountId,
    ) -> DispatchResult {
        match share_id {
            ShareID::Flat(sid) => {
                <<T as Trait>::FlatShareData as SubGroupSupervisorPermissions<
                    u32,
                    u32,
                    <T as frame_system::Trait>::AccountId,
                >>::set_sub_group_supervisor(organization, sid, setter, new)
            }
            ShareID::WeightedAtomic(sid) => {
                <<T as Trait>::WeightedShareData as SubGroupSupervisorPermissions<
                    u32,
                    u32,
                    <T as frame_system::Trait>::AccountId,
                >>::set_sub_group_supervisor(organization, sid, setter, new)
            }
        }
    }
}

impl<T: Trait> FlatShareWrapper<u32, u32, T::AccountId> for Module<T> {
    fn get_flat_share_group(
        organization: u32,
        share_id: u32,
    ) -> Result<Vec<T::AccountId>, DispatchError> {
        let ret = <<T as Trait>::FlatShareData as GetFlatShareGroup<
            <T as frame_system::Trait>::AccountId,
        >>::get_organization_share_group(organization, share_id)
        .ok_or(Error::<T>::FlatShareGroupNotFound)?;
        Ok(ret)
    }
    fn generate_unique_flat_share_id(organization: u32) -> u32 {
        <<T as Trait>::FlatShareData as SeededGenerateUniqueID<u32, u32>>::generate_unique_id(
            organization,
        )
    }
    fn add_members_to_flat_share_group(
        organization: u32,
        share_id: u32,
        members: Vec<T::AccountId>,
    ) {
        let prefix = UUID2::new(organization, share_id);
        <<T as Trait>::FlatShareData as ChangeGroupMembership<
            <T as frame_system::Trait>::AccountId,
        >>::batch_add_group_members(prefix, members);
    }
}

impl<T: Trait> WeightedShareWrapper<u32, u32, T::AccountId> for Module<T> {
    type Shares = SharesOf<T>; // exists only to pass inheritance to modules that inherit org
    type Profile = <<T as Trait>::WeightedShareData as WeightedShareGroup<
        <T as frame_system::Trait>::AccountId,
    >>::Profile;
    type Genesis = SimpleShareGenesis<T::AccountId, Self::Shares>;
    fn get_member_share_profile(
        organization: u32,
        share_id: u32,
        member: &T::AccountId,
    ) -> Option<Self::Profile> {
        <<T as Trait>::WeightedShareData as WeightedShareGroup<
            <T as frame_system::Trait>::AccountId,
        >>::get_share_profile(organization, share_id, member)
    }
    fn get_weighted_share_group(
        organization: u32,
        share_id: u32,
    ) -> Result<Self::Genesis, DispatchError> {
        let ret = <<T as Trait>::WeightedShareData as WeightedShareGroup<
            <T as frame_system::Trait>::AccountId,
        >>::shareholder_membership(organization, share_id)
        .ok_or(Error::<T>::WeightedShareGroupNotFound)?;
        Ok(ret.into())
    }
    fn get_outstanding_weighted_shares(organization: u32, share_id: u32) -> Option<Self::Shares> {
        <<T as Trait>::WeightedShareData as WeightedShareGroup<
            <T as frame_system::Trait>::AccountId,
        >>::outstanding_shares(organization, share_id)
    }
    fn generate_unique_weighted_share_id(organization: u32) -> u32 {
        <<T as Trait>::WeightedShareData as SeededGenerateUniqueID<u32, u32>>::generate_unique_id(
            organization,
        )
    }
}

impl<T: Trait> WeightedShareIssuanceWrapper<u32, u32, T::AccountId, Permill> for Module<T> {
    fn issue_weighted_shares_from_accounts(
        organization: u32,
        members: Vec<(T::AccountId, Self::Shares)>,
    ) -> Result<u32, DispatchError> {
        let share_id = Self::generate_unique_weighted_share_id(organization);
        <<T as Trait>::WeightedShareData as ShareBank<
            <T as frame_system::Trait>::AccountId,
        >>::batch_issue(organization, share_id, members.into())?;
        Ok(share_id)
    }
    fn burn_weighted_shares_for_member(
        organization: u32,
        share_id: u32,
        account: T::AccountId,
        // TODO: make portion an enum that expresses amount or permill
        // execute logic in here to decide that amount
        amount_to_burn: Option<Permill>,
    ) -> Result<Self::Shares, DispatchError> {
        let total_shares = Self::get_member_share_profile(organization, share_id, &account)
            .ok_or(Error::<T>::NoProfileFoundForAccountToBurn)?
            .total();
        let shares_to_burn = if let Some(pct_2_burn) = amount_to_burn {
            pct_2_burn * total_shares
        } else {
            total_shares
        };
        <<T as Trait>::WeightedShareData as ShareBank<<T as frame_system::Trait>::AccountId>>::burn(
            organization,
            share_id,
            account,
            shares_to_burn.clone(),
            false,
        )?;
        Ok(shares_to_burn)
    }
}

impl<T: Trait> RegisterShareGroup<u32, u32, T::AccountId, SharesOf<T>> for Module<T> {
    fn register_inner_flat_share_group(
        organization: u32,
        group: Vec<T::AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let raw_share_id = Self::generate_unique_flat_share_id(organization);
        Self::add_members_to_flat_share_group(organization, raw_share_id, group);
        let new_share_id = ShareID::Flat(raw_share_id);
        OrganizationInnerShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_inner_weighted_share_group(
        organization: u32,
        group: Vec<(T::AccountId, SharesOf<T>)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let raw_share_id = Self::issue_weighted_shares_from_accounts(organization, group)?;
        let new_share_id = ShareID::WeightedAtomic(raw_share_id);
        OrganizationInnerShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_outer_flat_share_group(
        organization: u32,
        group: Vec<T::AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let raw_share_id = Self::generate_unique_flat_share_id(organization);
        Self::add_members_to_flat_share_group(organization, raw_share_id, group);
        let new_share_id = ShareID::Flat(raw_share_id);
        OrganizationOuterShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_outer_weighted_share_group(
        organization: u32,
        group: Vec<(T::AccountId, SharesOf<T>)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let raw_share_id = Self::issue_weighted_shares_from_accounts(organization, group)?;
        let new_share_id = ShareID::WeightedAtomic(raw_share_id);
        OrganizationOuterShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
}

impl<T: Trait> GetInnerOuterShareGroups<u32, T::AccountId> for Module<T> {
    fn get_inner_share_group_identifiers(
        organization: u32,
    ) -> Option<Vec<Self::MultiShareIdentifier>> {
        let inner_shares = <OrganizationInnerShares>::iter()
            .filter(|(org, _, exists)| (org == &organization) && *exists)
            .map(|(_, share_id, _)| share_id)
            .collect::<Vec<_>>();
        if inner_shares.is_empty() {
            None
        } else {
            Some(inner_shares)
        }
    }
    fn get_outer_share_group_identifiers(
        organization: u32,
    ) -> Option<Vec<Self::MultiShareIdentifier>> {
        let outer_shares = <OrganizationOuterShares>::iter()
            .filter(|(org, _, exists)| (org == &organization) && *exists)
            .map(|(_, share_id, _)| share_id)
            .collect::<Vec<_>>();
        if outer_shares.is_empty() {
            None
        } else {
            Some(outer_shares)
        }
    }
}

impl<T: Trait> OrganizationDNS<u32, T::AccountId, IpfsReference> for Module<T> {
    type OrgSrc = OrganizationSource<T::AccountId, SharesOf<T>>;
    type OrganizationState = Organization<IpfsReference>;
    /// This method registers all the necessary ShareId and Power Structures from `Self::OrgSrc` and returns the `OrganizationState`
    /// which contains all the identifiers to track all of those power structures independently within the inherited modules
    fn organization_from_src(
        src: Self::OrgSrc,
        organization: u32,
        value_constitution: IpfsReference,
    ) -> Result<Self::OrganizationState, DispatchError> {
        let share_id = match src {
            OrganizationSource::Accounts(unweighted_members) => {
                // register shares membership group with this membership set
                let new_share_id = Self::generate_unique_flat_share_id(organization);
                Self::add_members_to_flat_share_group(
                    organization,
                    new_share_id,
                    unweighted_members,
                );
                ShareID::Flat(new_share_id)
            }
            OrganizationSource::AccountsWeighted(weighted_members) => {
                let raw_share_id =
                    Self::issue_weighted_shares_from_accounts(organization, weighted_members)?;
                ShareID::WeightedAtomic(raw_share_id)
            }
            OrganizationSource::SpinOffShareGroup(org_id, share_id) => {
                // check existence of the share group with this identifier and return the variant
                let existence_check = Self::check_share_group_existence(org_id, share_id);
                ensure!(
                    existence_check,
                    Error::<T>::SpinOffCannotOccurFromNonExistentShareGroup
                );
                match share_id {
                    ShareID::Flat(sid) => {
                        // register it as a new organization and set it as the set
                        let new_members = Self::get_flat_share_group(org_id, sid)?;
                        let new_share_id = Self::generate_unique_flat_share_id(organization);
                        Self::add_members_to_flat_share_group(
                            organization,
                            new_share_id,
                            new_members,
                        );
                        ShareID::Flat(new_share_id)
                    }
                    ShareID::WeightedAtomic(sid) => {
                        let new_weighted_membership =
                            Self::get_weighted_share_group(org_id, sid)?.account_ownership();
                        let raw_share_id = Self::issue_weighted_shares_from_accounts(
                            organization,
                            new_weighted_membership,
                        )?;
                        ShareID::WeightedAtomic(raw_share_id)
                    }
                }
            }
        };
        // TODO: symmetric API for deletion (removal and state gc)
        OrganizationInnerShares::insert(organization, share_id, true);
        // use it to create an organization
        let new_organization = Organization::new(share_id, value_constitution);
        Ok(new_organization)
    }
    fn register_organization(
        source: Self::OrgSrc,
        value_constitution: IpfsReference,
        supervisor: Option<T::AccountId>,
    ) -> Result<(u32, Self::OrganizationState), DispatchError> {
        let new_org_id = <<T as Trait>::OrgData as GenerateUniqueID<u32>>::generate_unique_id();
        // use helper method to register everything and return main storage item for tracking associated state/permissions for org
        let new_organization_state =
            Self::organization_from_src(source, new_org_id, value_constitution)?;
        // insert _canonical_ state of the organization
        OrganizationStates::insert(new_org_id, new_organization_state.clone());
        // assign the supervisor to supervisor if that assignment is specified
        if let Some(org_sudo) = supervisor {
            Self::put_organization_supervisor(new_org_id, org_sudo);
        }
        // iterate the OrganizationCounter
        let new_counter = OrganizationCounter::get() + 1u32;
        OrganizationCounter::put(new_counter);
        Ok((new_org_id, new_organization_state))
    }
}
