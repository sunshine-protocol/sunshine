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
    share::{ShareProfile, SimpleShareGenesis},
    traits::{
        AccessGenesis, AccessProfile, ChainSudoPermissions, ChangeGroupMembership,
        FlatShareWrapper, GenerateUniqueID, GetGroup, GetGroupSize, GetInnerOuterShareGroups,
        GroupMembership, IDIsAvailable, LockProfile, OrgChecks, OrganizationDNS,
        OrganizationSupervisorPermissions, RegisterShareGroup, ReserveProfile,
        SeededGenerateUniqueID, ShareBank, ShareGroupChecks, ShareInformation, ShareIssuance,
        SupervisorPermissions, VerifyShape, WeightedShareGroup, WeightedShareIssuanceWrapper,
        WeightedShareWrapper,
    },
    uuid::ShareGroup,
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
    traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, CheckedAdd, CheckedSub, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult, Permill,
};
use sp_std::{fmt::Debug, prelude::*};

pub trait Trait: system::Trait {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid
    type IpfsReference: Parameter + Member;

    /// Organizational identifier
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

    /// Metric for ownership in the context of OrgId
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
    /// - why? we need to track how much the group check is called and limit it somehow and this is the best I've come up with for now...TODO: make issue and get feedback
    type ReservationLimit: Get<u32>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        OrgId = <T as Trait>::OrgId,
        <T as Trait>::Shares,
        <T as Trait>::IpfsReference,
    {
        // ~~ FLAT, BASE ORG EVENTS ~~
        /// Organization ID, New Member Account ID, Amount Issued
        NewMemberAddedToOrg(OrgId, AccountId, Shares),
        /// Organization ID, Old Member Account Id, Amount Burned
        OldMemberRemovedFromOrg(OrgId, AccountId, Shares),
        /// Batch Addition by the Account ID, _,  total shares added
        BatchMemberAdditionForOrg(AccountId, OrgId, Shares),
        /// Batch Removal by the Account ID, _, total shares burned
        BatchMemberRemovalForOrg(AccountId, OrgId, Shares),
        /// Registrar Account ID, _, Constitution reference and the total shares issued for this OrgId
        NewOrganizationRegistered(AccountId, OrgId, IpfsReference, Shares),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        UnAuthorizedSwapSudoRequest,
        NoExistingSudoKey,
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
        // --
        OrganizationMustExistToClearSupervisor,
        OrganizationMustExistToPutSupervisor,
        CannotAddAnExistingMemberToOrg,
        CannotRemoveNonMemberFromOrg,
        // --
        CannotBurnMoreThanTotalIssuance,
        NotEnoughSharesToSatisfyBurnRequest,
        IssuanceCannotGoNegative,
        GenesisTotalMustEqualSumToUseBatchOps,
        IssuanceWouldOverflowShares,
        /// If this is thrown, an invariant is broken and the alarm comes from `remove_member_from_org`
        IssuanceGoesNegativeWhileRemovingMember,
        CannotReserveMoreThanShareTotal,
        ReservationWouldExceedHardLimit,
        CannotUnReserveMoreThanShareTotal,
        CannotLockIfAlreadyLocked,
        CannotUnLockIfAlreadyUnLocked,
        CannotUnLockProfileThatDNE,
        CannotLockProfileThatDNE,
        CannotReserveIfMemberProfileDNE,
        CannotUnReserveIfMemberProfileDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Org {
        /// The account that can set all the organization supervisors, should be replaced by committee-based governance
        SudoKey get(fn sudo_key): Option<T::AccountId>;

        /// Identity nonce for registering organizations
        OrgIdNonce get(fn org_id_counter): T::OrgId;

        /// The main storage item for Organization registration
        OrganizationStates get(fn organization_states): map
            hasher(opaque_blake2_256) T::OrgId => Option<Organization<T::AccountId, T::OrgId, T::IpfsReference>>;

        /// The map to track organizational membership
        Members get(fn members): double_map
            hasher(blake2_128_concat) T::OrgId,
            hasher(blake2_128_concat) T::AccountId => Option<ShareProfile<T::Shares>>;

        /// Total number of outstanding shares that express relative ownership in group
        TotalIssuance get(fn total_issuance): map
            hasher(opaque_blake2_256) T::OrgId => T::Shares;

        /// The size for each organization
        OrganizationSize get(fn organization_size): map hasher(opaque_blake2_256) T::OrgId => u32;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_org(
            origin,
            sudo: Option<T::AccountId>,
            parent_org: Option<T::OrgId>,
            constitution: T::IpfsReference,
            genesis: SimpleShareGenesis<T::AccountId, T::Shares>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // TODO: auth based on parent_org and/or membership in 0 org
            // TODO: move this into a trait? yes
            let new_org = Organization::new(sudo, parent_org, constitution.clone());
            let new_id = Self::generate_unique_id();
            <OrganizationStates<T>>::insert(new_id, new_org);
            Self::deposit_event(RawEvent::NewOrganizationRegistered(caller, new_id, constitution, T::Shares::zero()));
            Ok(())
        }
        #[weight = 0]
        fn add_new_member_to_org(
            origin,
            organization: T::OrgId,
            new_member: T::AccountId,
            shares_issued: T::Shares,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::add_member_to_org(organization, new_member.clone(), false)?;
            Self::deposit_event(RawEvent::NewMemberAddedToOrg(organization, new_member, T::Shares::zero()));
            Ok(())
        }
        #[weight = 0]
        fn remove_old_member_from_org(
            origin,
            organization: T::OrgId,
            old_member: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::remove_member_from_org(organization, old_member.clone(), false)?;
            Self::deposit_event(RawEvent::OldMemberRemovedFromOrg(organization, old_member, T::Shares::zero()));
            Ok(())
        }
        #[weight = 0]
        fn add_new_members_to_org(
            origin,
            organization: T::OrgId,
            new_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_add_members_to_org(organization, new_members)?;
            Self::deposit_event(RawEvent::BatchMemberAdditionForOrg(caller, organization, T::Shares::zero()));
            Ok(())
        }
        #[weight = 0]
        fn remove_old_members_from_org(
            origin,
            organization: T::OrgId,
            old_members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_key(&caller) || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeMembership);

            Self::batch_remove_members_from_org(organization, old_members)?;
            Self::deposit_event(RawEvent::BatchMemberRemovalForOrg(caller, organization, T::Shares::zero()));
            Ok(())
        }
    }
}

impl<T: Trait> GetGroupSize<T::OrgId> for Module<T> {
    fn get_size_of_group(org_id: T::OrgId) -> u32 {
        <OrganizationSize<T>>::get(org_id)
    }
}

impl<T: Trait> GroupMembership<T::OrgId, T::AccountId> for Module<T> {
    fn is_member_of_group(org_id: T::OrgId, who: &T::AccountId) -> bool {
        <Members<T>>::get(org_id, who).is_some()
    }
}

impl<T: Trait> IDIsAvailable<T::OrgId> for Module<T> {
    fn id_is_available(id: T::OrgId) -> bool {
        <OrganizationStates<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<T::OrgId> for Module<T> {
    fn generate_unique_id() -> T::OrgId {
        let mut id_counter = <OrgIdNonce<T>>::get() + 1u32.into();
        while <OrganizationStates<T>>::get(id_counter).is_some() {
            // add overflow check here? not really necessary
            id_counter += 1u32.into();
        }
        <OrgIdNonce<T>>::put(id_counter);
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
        if let Some(state) = <OrganizationStates<T>>::get(org) {
            return state.is_sudo(who);
        }
        false
    }
    /// Removes any existing sudo and places None
    fn clear_organization_supervisor(org: T::OrgId) -> DispatchResult {
        let old_org = <OrganizationStates<T>>::get(org)
            .ok_or(Error::<T>::OrganizationMustExistToClearSupervisor)?;
        let new_org = old_org.clear_sudo();
        <OrganizationStates<T>>::insert(org, new_org);
        Ok(())
    }
    /// Removes any existing sudo and places `who`
    fn put_organization_supervisor(org: T::OrgId, who: T::AccountId) -> DispatchResult {
        let old_org = <OrganizationStates<T>>::get(org)
            .ok_or(Error::<T>::OrganizationMustExistToPutSupervisor)?;
        let new_org = old_org.put_sudo(who);
        <OrganizationStates<T>>::insert(org, new_org);
        Ok(())
    }
}

impl<T: Trait> ChangeGroupMembership<T::OrgId, T::AccountId> for Module<T> {
    fn add_member_to_org(
        org_id: T::OrgId,
        new_member: T::AccountId,
        batch: bool,
    ) -> DispatchResult {
        // prevent adding duplicate members
        ensure!(
            <Members<T>>::get(org_id, &new_member).is_none(),
            Error::<T>::CannotAddAnExistingMemberToOrg
        );
        if !batch {
            let new_organization_size = <OrganizationSize<T>>::get(org_id) + 1u32;
            <OrganizationSize<T>>::insert(org_id, new_organization_size);
        }
        // default 0 share profile inserted for this method
        <Members<T>>::insert(org_id, new_member, ShareProfile::default());
        Ok(())
    }
    fn remove_member_from_org(
        org_id: T::OrgId,
        old_member: T::AccountId,
        batch: bool,
    ) -> DispatchResult {
        // prevent removal of non-member
        let current_profile = <Members<T>>::get(org_id, &old_member)
            .ok_or(Error::<T>::CannotRemoveNonMemberFromOrg)?;
        // update issuance if it changes due to this removal
        if current_profile.total() > T::Shares::zero() {
            let new_issuance = <TotalIssuance<T>>::get(org_id)
                .checked_sub(&current_profile.total())
                .ok_or(Error::<T>::IssuanceGoesNegativeWhileRemovingMember)?;
            <TotalIssuance<T>>::insert(org_id, new_issuance);
        }
        if !batch {
            let new_organization_size = <OrganizationSize<T>>::get(org_id).saturating_sub(1u32);
            <OrganizationSize<T>>::insert(org_id, new_organization_size);
        }
        <Members<T>>::remove(org_id, old_member);
        Ok(())
    }
    // WARNING: the vector fed as inputs to the following methods must have NO duplicates
    fn batch_add_members_to_org(
        org_id: T::OrgId,
        new_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        let size_increase: u32 = new_members.len() as u32;
        let new_organization_size: u32 =
            <OrganizationSize<T>>::get(org_id).saturating_add(size_increase);
        <OrganizationSize<T>>::insert(org_id, new_organization_size);
        new_members
            .into_iter()
            .map(|member| Self::add_member_to_org(org_id, member, true))
            .collect::<DispatchResult>()
    }
    fn batch_remove_members_from_org(
        org_id: T::OrgId,
        old_members: Vec<T::AccountId>,
    ) -> DispatchResult {
        let size_decrease: u32 = old_members.len() as u32;
        let new_organization_size: u32 =
            <OrganizationSize<T>>::get(org_id).saturating_sub(size_decrease);
        <OrganizationSize<T>>::insert(org_id, new_organization_size);
        old_members
            .into_iter()
            .map(|member| Self::remove_member_from_org(org_id, member, true))
            .collect::<DispatchResult>()
    }
}

impl<T: Trait> GetGroup<T::OrgId, T::AccountId> for Module<T> {
    fn get_group(organization: T::OrgId) -> Option<Vec<T::AccountId>> {
        if !Self::id_is_available(organization) {
            Some(
                // is this the same performance as a get() with Key=Vec<AccountId>
                <Members<T>>::iter()
                    .filter(|(org, _, _)| *org == organization)
                    .map(|(_, account, _)| account)
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        }
    }
}

impl<T: Trait> ShareInformation<T::OrgId, T::AccountId, T::Shares> for Module<T> {
    type Profile = ShareProfile<T::Shares>;
    type Genesis = SimpleShareGenesis<T::AccountId, T::Shares>;
    /// Gets the total number of shares issued for an organization's share identifier
    fn outstanding_shares(organization: T::OrgId) -> T::Shares {
        <TotalIssuance<T>>::get(organization)
    }
    /// Get input's share profile if it exists
    fn get_share_profile(organization: T::OrgId, who: &T::AccountId) -> Option<Self::Profile> {
        <Members<T>>::get(organization, who)
    }
    /// Returns the entire membership group associated with a share identifier, fallible bc checks existence
    fn get_membership_with_shape(organization: T::OrgId) -> Option<Self::Genesis> {
        if !Self::id_is_available(organization) {
            Some(
                // is this the same performance as a get() with Key=Vec<AccountId>
                <Members<T>>::iter()
                    .filter(|(org, _, _)| *org == organization)
                    .map(|(_, account, profile)| (account, profile.total()))
                    .collect::<Vec<(T::AccountId, T::Shares)>>()
                    .into(),
            )
        } else {
            None
        }
    }
}
impl<T: Trait> ShareIssuance<T::OrgId, T::AccountId, T::Shares> for Module<T> {
    // TODO: change to infallible instead of returning DispatchResult?
    fn issue(
        organization: T::OrgId,
        new_owner: T::AccountId,
        amount: T::Shares,
        batch: bool,
    ) -> DispatchResult {
        let new_profile =
            if let Some(existing_profile) = <Members<T>>::get(organization, &new_owner) {
                existing_profile.add_shares(amount)
            } else {
                ShareProfile::new_shares(amount)
            };
        if !batch {
            let new_issuance = <TotalIssuance<T>>::get(organization) + amount;
            <TotalIssuance<T>>::insert(organization, new_issuance);
        }
        <Members<T>>::insert(organization, new_owner, new_profile);
        Ok(())
    }
    // does not necessarily leave ownership, unless the new amount is 0
    fn burn(
        organization: T::OrgId,
        old_owner: T::AccountId,
        amount: T::Shares,
        batch: bool,
    ) -> DispatchResult {
        let old_profile = <Members<T>>::get(organization, &old_owner)
            .ok_or(Error::<T>::NotEnoughSharesToSatisfyBurnRequest)?;
        let old_issuance = <TotalIssuance<T>>::get(organization);
        ensure!(
            old_issuance >= amount,
            Error::<T>::CannotBurnMoreThanTotalIssuance
        );
        ensure!(
            old_profile.total() >= amount,
            Error::<T>::NotEnoughSharesToSatisfyBurnRequest
        );
        if !batch {
            let new_issuance = old_issuance - amount;
            <TotalIssuance<T>>::insert(organization, new_issuance);
        }
        let new_profile = old_profile.subtract_shares(amount);
        if new_profile.is_zero() {
            // leave the group
            <Members<T>>::remove(organization, old_owner);
        } else {
            <Members<T>>::insert(organization, old_owner, new_profile);
        }
        Ok(())
    }
    fn batch_issue(organization: T::OrgId, genesis: Self::Genesis) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance = <TotalIssuance<T>>::get(organization);
        let new_issuance = old_issuance
            .checked_add(&genesis.total())
            .ok_or(Error::<T>::IssuanceWouldOverflowShares)?;
        genesis
            .account_ownership()
            .into_iter()
            .map(|(member, shares)| -> DispatchResult {
                Self::issue(organization, member, shares, true)
            })
            .collect::<DispatchResult>()?;
        <TotalIssuance<T>>::insert(organization, new_issuance);
        Ok(())
    }
    fn batch_burn(organization: T::OrgId, genesis: Self::Genesis) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance = <TotalIssuance<T>>::get(organization);
        let new_issuance = old_issuance
            .checked_sub(&genesis.total())
            .ok_or(Error::<T>::IssuanceCannotGoNegative)?;
        genesis
            .account_ownership()
            .into_iter()
            .map(|(member, shares)| -> DispatchResult {
                Self::issue(organization, member, shares, true)
            })
            .collect::<DispatchResult>()?;
        <TotalIssuance<T>>::insert(organization, new_issuance);
        Ok(())
    }
}
impl<T: Trait> ReserveProfile<T::OrgId, T::AccountId, T::Shares> for Module<T> {
    fn reserve(
        organization: T::OrgId,
        who: &T::AccountId,
        amount: Option<T::Shares>,
    ) -> Result<T::Shares, DispatchError> {
        let old_profile = <Members<T>>::get(organization, who)
            .ok_or(Error::<T>::CannotReserveIfMemberProfileDNE)?;
        let amount_to_reserve = if let Some(amt) = amount {
            ensure!(
                amt >= old_profile.total(),
                Error::<T>::CannotReserveMoreThanShareTotal
            );
            amt
        } else {
            old_profile.total()
        };
        // increment times_reserved
        let times_reserved = old_profile.times_reserved() + 1u32;
        // make sure it's below the hard reservation limit
        ensure!(
            times_reserved < T::ReservationLimit::get(),
            Error::<T>::ReservationWouldExceedHardLimit
        );
        // instantiate new share profile which just iterates times_reserved
        let new_share_profile = old_profile.increment_times_reserved();
        <Members<T>>::insert(organization, who, new_share_profile);
        Ok(amount_to_reserve)
    }
    fn unreserve(
        organization: T::OrgId,
        who: &T::AccountId,
        amount: Option<T::Shares>,
    ) -> Result<T::Shares, DispatchError> {
        let old_profile = <Members<T>>::get(organization, who)
            .ok_or(Error::<T>::CannotUnReserveIfMemberProfileDNE)?;
        let amount_to_unreserve = if let Some(amt) = amount {
            ensure!(
                amt >= old_profile.total(),
                Error::<T>::CannotUnReserveMoreThanShareTotal
            );
            amt
        } else {
            old_profile.total()
        };
        // make sure there is some existing reservation which is being closed
        ensure!(
            old_profile.times_reserved() > 0,
            Error::<T>::ReservationWouldExceedHardLimit
        );
        // instantiate new share profile which just iterates times_reserved
        let new_share_profile = old_profile.decrement_times_reserved();
        <Members<T>>::insert(organization, who, new_share_profile);
        Ok(amount_to_unreserve)
    }
}
// TODO: enforce lock on other actions in this module?
// -> burning shares is not allowed?
// -> reserving shares is not allowed?
impl<T: Trait> LockProfile<T::OrgId, T::AccountId> for Module<T> {
    fn lock_profile(organization: T::OrgId, who: &T::AccountId) -> DispatchResult {
        let old_profile =
            <Members<T>>::get(organization, who).ok_or(Error::<T>::CannotLockProfileThatDNE)?;
        ensure!(
            old_profile.is_unlocked(),
            Error::<T>::CannotLockIfAlreadyLocked
        );
        let new_profile = old_profile.lock();
        <Members<T>>::insert(organization, who, new_profile);
        Ok(())
    }
    fn unlock_profile(organization: T::OrgId, who: &T::AccountId) -> DispatchResult {
        let old_profile =
            <Members<T>>::get(organization, who).ok_or(Error::<T>::CannotUnLockProfileThatDNE)?;
        ensure!(
            !old_profile.is_unlocked(),
            Error::<T>::CannotUnLockIfAlreadyUnLocked
        );
        let new_profile = old_profile.unlock();
        <Members<T>>::insert(organization, who, new_profile);
        Ok(())
    }
}
