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
        AccessGenesis, ChangeGroupMembership, GenerateUniqueID, GetFlatShareGroup, GetGroupSize,
        GroupMembership, IDIsAvailable, LockableProfile, OrgChecks, OrganizationDNS,
        RegisterShareGroup, ReservableProfile, ShareBank, ShareGroupChecks,
        SubSupervisorKeyManagement, SudoKeyManagement, SupervisorKeyManagement, WeightedShareGroup,
    },
    uuid::UUID2,
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
    Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{CheckedSub, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::WeightedShareData as WeightedShareGroup<
    <T as frame_system::Trait>::AccountId,
>>::Shares;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    // notably not AtLeast32Bit...
    type OrgId: Parameter
        + Member
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + CheckedSub
        + Zero
        + From<u32>
        + Into<u32>; // TODO: replace with + From<OrgId<Self>> s.t. OrgId<T>: <OrgData as _::OrgId> once that is added back
    type FlatShareId: Parameter
        + Member
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + CheckedSub
        + Zero
        + From<u32>
        + Into<u32>; // + From<FlatShareId<Self>> s.t. FlatShareId<T>: <FlatShareData as _::ShareId> once that is added back
    type WeightedShareId: Parameter
        + Member
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + CheckedSub
        + Zero
        + From<u32>
        + Into<u32>; // + From<WeightedShareId<Self>> s.t. WeightedShareId<T>: <WeightedShareData as _::ShareId> once that is added back

    /// Used for permissions and shared for organizational membership in both
    /// shares modules
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<u32>
        + GenerateUniqueID<u32>
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
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Org {
        /// The number of organizations in this module (eventually add nuance like `Active` tiers)
        OrganizationCounter get(fn organization_counter): u32;

        OrganizationIdentityNonce get(fn organization_identity_nonce): u32;

        // OrgId => FlateShareIDNonce
        FlatShareIDNonce get(fn flat_share_id_nonce): map
            hasher(opaque_blake2_256) u32 => u32;

        // OrgId => WeightedShareIDNonce
        WeightedShareIDNonce get(fn weighted_share_id_nonce): map
            hasher(opaque_blake2_256) u32 => u32;

        /// Organizations that were registered in this module
        OrganizationStates get(fn organization_states): map
            hasher(opaque_blake2_256) u32 => Option<Organization<IpfsReference>>;

        /// Organization Inner Shares
        /// - purpose: organizational governance, task supervision
        OrganizationInnerShares get(fn organization_inner_shares): double_map
            hasher(opaque_blake2_256) u32,
            hasher(opaque_blake2_256) ShareID => bool;

        /// Organization Outer Shares
        /// - purpose: funded teams ownership enforcement, external contractors
        OrganizationOuterShares get(fn organization_outer_shares): double_map
            hasher(opaque_blake2_256) u32,
            hasher(opaque_blake2_256) ShareID => bool;
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
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_membership_in_org(1u32.into(), &caller);
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
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller);
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
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller);
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
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterOuterShares);

            let new_share_id = Self::register_outer_weighted_share_group(organization, group)?;
            Self::deposit_event(RawEvent::WeightedOuterShareGroupAddedToOrg(caller, organization, new_share_id));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // $$$ AUTH CHECKS $$$
    fn check_if_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as SudoKeyManagement<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
    }
    fn check_if_organization_supervisor_account(organization: u32, who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as SupervisorKeyManagement<<T as frame_system::Trait>::AccountId>>::is_organization_supervisor(organization, who)
    }
    // fn check_if_sub_organization_supervisor_account_for_flat_shares(
    //     organization: OrgId,
    //     share_id: u32,
    //     who: &T::AccountId,
    // ) -> bool {
    //     <<T as Trait>::FlatShareData as SubSupervisorKeyManagement<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::is_sub_organization_supervisor(organization, share_id, who)
    // }
    // fn check_if_sub_organization_supervisor_account_for_weighted_shares(
    //     organization: OrgId,
    //     share_id: u32,
    //     who: &T::AccountId,
    // ) -> bool {
    //     <<T as Trait>::WeightedShareData as SubSupervisorKeyManagement<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::is_sub_organization_supervisor(organization, share_id, who)
    // }
    fn set_organization_supervisor(organization: u32, who: T::AccountId) -> DispatchResult {
        <<T as Trait>::OrgData as SupervisorKeyManagement<<T as frame_system::Trait>::AccountId>>::set_supervisor(organization, who)
    }
    // fn set_flat_share_group_supervisor(
    //     organization: OrgId,
    //     share_id: u32,
    //     who: T::AccountId,
    // ) -> DispatchResult {
    //     <<T as Trait>::FlatShareData as SubSupervisorKeyManagement<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::set_sub_supervisor(organization, share_id, who)
    // }
    // fn set_weighted_share_group_supervisor(
    //     organization: OrgId,
    //     share_id: u32,
    //     who: T::AccountId,
    // ) -> DispatchResult {
    //     <<T as Trait>::WeightedShareData as SubSupervisorKeyManagement<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::set_sub_supervisor(organization, share_id, who)
    // }
    // helpers for shares-membership (FlatShareData)
    fn generate_unique_flat_share_id(organization: u32) -> ShareID {
        // TODO: delete FlatShareID Nonce in this module
        let generated_joint_id =
            <<T as Trait>::FlatShareData as GenerateUniqueID<UUID2>>::generate_unique_id(
                UUID2::new(organization, 1u32),
            );
        ShareID::Flat(generated_joint_id.two())
    }
    fn add_members_to_flat_share_group(
        organization: u32,
        share_id: ShareID,
        members: Vec<T::AccountId>,
    ) -> DispatchResult {
        match share_id {
            ShareID::Flat(sid) => {
                let prefix = UUID2::new(organization, sid);
                <<T as Trait>::FlatShareData as ChangeGroupMembership<
                    <T as frame_system::Trait>::AccountId,
                >>::batch_add_group_members(prefix, members);
                Ok(())
            }
            _ => Err(Error::<T>::ShareIdTypeNotShareMembershipVariantSoCantAddMembers.into()),
        }
    }
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
    // helpers for  `shares-atomic`
    fn generate_unique_weighted_share_id(organization: u32) -> u32 {
        let new_nonce = WeightedShareIDNonce::get(organization) + 1;
        let new_joint_id = UUID2::new(organization, new_nonce);
        FlatShareIDNonce::insert(organization, new_nonce);
        let generated_joint_id =
            <<T as Trait>::WeightedShareData as GenerateUniqueID<UUID2>>::generate_unique_id(
                new_joint_id,
            );
        generated_joint_id.two()
    }
    fn register_weighted_share_group(
        organization: u32,
        members: Vec<(T::AccountId, SharesOf<T>)>,
    ) -> Result<ShareID, DispatchError> {
        let share_id = Self::generate_unique_weighted_share_id(organization);
        <<T as Trait>::WeightedShareData as ShareBank<
            <T as frame_system::Trait>::AccountId,
        >>::batch_issue(organization, share_id, members.into())?;
        Ok(ShareID::WeightedAtomic(share_id))
    }
    // fn get_weighted_shares_for_member(
    //     organization: u32,
    //     share_id: u32,
    //     member: &T::AccountId,
    // ) -> Result<SharesOf<T>, DispatchError> {
    //     <<T as Trait>::WeightedShareData as WeightedShareGroup<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::get_share_profile(organization, share_id, member)
    // }
    fn get_weighted_share_group(
        organization: u32,
        share_id: u32,
    ) -> Result<SimpleShareGenesis<T::AccountId, SharesOf<T>>, DispatchError> {
        let ret = <<T as Trait>::WeightedShareData as WeightedShareGroup<
            <T as frame_system::Trait>::AccountId,
        >>::shareholder_membership(organization, share_id)
        .ok_or(Error::<T>::WeightedShareGroupNotFound)?;
        Ok(ret.into())
    }
    // fn get_outstanding_weighted_shares(
    //     organization: u32,
    //     share_id: u32,
    // ) -> Result<SharesOf<T>, DispatchError> {
    //     let ret = <<T as Trait>::WeightedShareData as WeightedShareGroup<
    //         <T as frame_system::Trait>::AccountId,
    //     >>::outstanding_shares(organization, share_id)
    //     .ok_or(Error::<T>::WeightedShareGroupNotFound)?;
    //     Ok(ret)
    // }
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
}

impl<T: Trait> RegisterShareGroup<u32, T::AccountId, SharesOf<T>> for Module<T> {
    fn register_inner_flat_share_group(
        organization: u32,
        group: Vec<T::AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let new_share_id = Self::generate_unique_flat_share_id(organization);
        Self::add_members_to_flat_share_group(organization, new_share_id, group)?;
        OrganizationInnerShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_inner_weighted_share_group(
        organization: u32,
        group: Vec<(T::AccountId, SharesOf<T>)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let new_share_id = Self::register_weighted_share_group(organization, group)?;
        OrganizationInnerShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_outer_flat_share_group(
        organization: u32,
        group: Vec<T::AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let new_share_id = Self::generate_unique_flat_share_id(organization);
        Self::add_members_to_flat_share_group(organization, new_share_id, group)?;
        OrganizationOuterShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
    }
    fn register_outer_weighted_share_group(
        organization: u32,
        group: Vec<(T::AccountId, SharesOf<T>)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError> {
        let new_share_id = Self::register_weighted_share_group(organization, group)?;
        OrganizationOuterShares::insert(organization, new_share_id, true);
        Ok(new_share_id)
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
                )?;
                new_share_id
            }
            OrganizationSource::AccountsWeighted(weighted_members) => {
                Self::register_weighted_share_group(organization, weighted_members)?
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
                        )?;
                        new_share_id
                    }
                    ShareID::WeightedAtomic(sid) => {
                        let new_weighted_membership =
                            Self::get_weighted_share_group(org_id, sid)?.account_ownership();
                        Self::register_weighted_share_group(organization, new_weighted_membership)?
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
        let new_org_id = <<T as Trait>::OrgData as GenerateUniqueID<u32>>::generate_unique_id(0u32);
        // use helper method to register everything and return main storage item for tracking associated state/permissions for org
        let new_organization_state =
            Self::organization_from_src(source, new_org_id, value_constitution)?;
        // insert _canonical_ state of the organization
        OrganizationStates::insert(new_org_id, new_organization_state.clone());
        // assign the supervisor to supervisor if that assignment is specified
        if let Some(org_sudo) = supervisor {
            Self::set_organization_supervisor(new_org_id, org_sudo)?;
        }
        // iterate the OrganizationCounter
        let new_counter = OrganizationCounter::get() + 1u32;
        OrganizationCounter::put(new_counter);
        Ok((new_org_id, new_organization_state))
    }
}
