#![recursion_limit = "256"]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! # Org Module
//! This module expresses a framework for configurable group governance
//!
//! - [`org::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet handles organization membership and governance.  Each
//! member (`AccountId`) in an org has some quantity of `Shares` in proportion
//! to their relative ownership and voting power.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html

#[cfg(test)]
mod tests;

use util::{
    organization::{
        Organization,
        OrganizationSource,
    },
    share::{
        ProfileState,
        SharePortion,
        ShareProfile,
        WeightedVector,
    },
    traits::{
        AccessGenesis,
        GenerateUniqueID,
        GetGroup,
        GroupMembership,
        IDIsAvailable,
        LockProfile,
        OrganizationSupervisorPermissions,
        RegisterOrganization,
        RemoveOrganization,
        ShareInformation,
        ShareIssuance,
        VerifyShape,
    },
};

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    storage::IterableStorageDoubleMap,
    Parameter,
};
use frame_system::{
    self as system,
    ensure_signed,
};
use orml_utilities::OrderedSet;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        AtLeast32BitUnsigned,
        CheckedAdd,
        CheckedSub,
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

type Org<T> = Organization<
    <T as frame_system::Trait>::AccountId,
    <T as Trait>::OrgId,
    <T as Trait>::Cid,
>;
type Profile<T> = ShareProfile<<T as Trait>::Shares, ProfileState>;

pub trait Trait: system::Trait {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid type
    type Cid: Parameter + Member + Default;

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
        + Zero
        + AtLeast32BitUnsigned;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        OrgId = <T as Trait>::OrgId,
        <T as Trait>::Shares,
        <T as Trait>::Cid,
    {
        /// No shares issued but an organization was registered with flat membership with the last `u32` as the number of members
        NewFlatOrganizationRegistered(AccountId, OrgId, Cid, u32),
        /// Shares issued for a weighted ownership org s.t. the last element `Shares` is total issuance
        NewWeightedOrganizationRegistered(AccountId, OrgId, Cid, Shares),
        /// Organization ID, New Member Account ID, Amount Issued
        NewMemberAddedToOrg(OrgId, AccountId, Shares),
        /// Organization ID, Old Member Account Id, Amount Burned
        OldMemberRemovedFromOrg(OrgId, AccountId, Shares),
        /// Batch Addition by the Account ID, _,  total shares added
        BatchMemberAdditionForOrg(AccountId, OrgId, Shares),
        /// Batch Removal by the Account ID, _, total shares burned
        BatchMemberRemovalForOrg(AccountId, OrgId, Shares),
        /// Organization ID, Account Id
        SharesLocked(OrgId, AccountId),
        /// Organization ID, Account Id
        SharesUnlocked(OrgId, AccountId),
        /// Organization ID, Recipient AccountId, Issued Amount
        SharesIssued(OrgId, AccountId, Shares),
        /// Organization ID, Burned AccountId, Burned Amount
        SharesBurned(OrgId, AccountId, Shares),
        /// Organization ID, Total Shares Minted
        SharesBatchIssued(OrgId, Shares),
        /// Organization ID, Total Shares Burned
        SharesBatchBurned(OrgId, Shares),
        /// Organization ID Removed
        OrganizationRemoved(OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        OrgDNE,
        UnAuthorizedSwapSudoRequest,
        NoExistingSudoKey,
        OrganizationMustExistToClearSupervisor,
        OrganizationMustExistToPutSupervisor,
        CannotBurnMoreThanTotalIssuance,
        NotEnoughSharesToSatisfyBurnRequest,
        IssuanceCannotGoNegative,
        GenesisTotalMustEqualSumToUseBatchOps,
        IssuanceWouldOverflowShares,
        IssuanceGoesNegativeWhileRemovingMember,
        CannotLockIfAlreadyLocked,
        CannotUnLockIfAlreadyUnLocked,
        CannotUnLockProfileThatDNE,
        CannotLockProfileThatDNE,
        OrganizationMustBeRegisteredToIssueShares,
        OrganizationMustBeRegisteredToBurnShares,
        OrganizationMustBeRegisteredToLockShares,
        OrganizationMustBeRegisteredToUnLockShares,
        NotAuthorizedToLockShares,
        NotAuthorizedToUnLockShares,
        NotAuthorizedToIssueShares,
        NotAuthorizedToBurnShares,
        OrganizationCannotBeRemovedIfInputIdIsAvailable,
        AccountHasNoOwnershipInOrg,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Org {
        /// Identity nonce for registering organizations
        OrgIdNonce get(fn org_id_counter): T::OrgId;

        /// The total number of organizations registered at any given time
        pub OrganizationCounter get(fn organization_counter): u32;

        /// The main storage item for Organization registration
        pub OrganizationStates get(fn organization_states): map
            hasher(blake2_128_concat) T::OrgId => Option<Org<T>>;

        /// Hierarchical relationships between orgs
        pub OrgHierarchy get(fn org_hierarchy): double_map
            hasher(blake2_128_concat) T::OrgId,
            hasher(blake2_128_concat) T::OrgId => Option<()>;

        /// The map to track organizational membership
        pub Members get(fn members): double_map
            hasher(blake2_128_concat) T::OrgId,
            hasher(blake2_128_concat) T::AccountId => Option<Profile<T>>;

        /// Total number of outstanding shares that express relative ownership in group
        pub TotalIssuance get(fn total_issuance): map
            hasher(opaque_blake2_256) T::OrgId => T::Shares;
    }
    add_extra_genesis {
        config(first_organization_supervisor): T::AccountId;
        config(first_organization_value_constitution): T::Cid;
        config(first_organization_flat_membership): Vec<T::AccountId>;

        build(|config: &GenesisConfig<T>| {
            <Module<T>>::register_flat_org(
                T::Origin::from(Some(config.first_organization_supervisor.clone()).into()),
                Some(config.first_organization_supervisor.clone()),
                None,
                config.first_organization_value_constitution.clone(),
                config.first_organization_flat_membership.clone(),
            ).expect("first organization config set up failed");
        })
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_flat_org(
            origin,
            sudo: Option<T::AccountId>,
            parent_org: Option<T::OrgId>,
            constitution: T::Cid,
            members: Vec<T::AccountId>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let total: u32 = members.len() as u32;
            let new_id = if let Some(parent_id) = parent_org {
                Self::register_sub_organization(parent_id, OrganizationSource::Accounts(members), sudo, constitution.clone())?
            } else {
                Self::register_organization(OrganizationSource::Accounts(members), sudo, constitution.clone())?
            };
            Self::deposit_event(RawEvent::NewFlatOrganizationRegistered(caller, new_id, constitution, total));
            Ok(())
        }
        #[weight = 0]
        fn register_weighted_org(
            origin,
            sudo: Option<T::AccountId>,
            parent_org: Option<T::OrgId>,
            constitution: T::Cid,
            weighted_members: Vec<(T::AccountId, T::Shares)>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // auth will usually be specific to the module context in which this is used
            let wm_cpy: WeightedVector<T::AccountId, T::Shares> = weighted_members.clone().into();
            let new_id = if let Some(parent_id) = parent_org {
                Self::register_sub_organization(parent_id, OrganizationSource::AccountsWeighted(weighted_members), sudo, constitution.clone())?
            } else {
                Self::register_organization(OrganizationSource::AccountsWeighted(weighted_members), sudo, constitution.clone())?
            };
            Self::deposit_event(RawEvent::NewWeightedOrganizationRegistered(caller, new_id, constitution, wm_cpy.total()));
            Ok(())
        }
        #[weight = 0]
        fn issue_shares(origin, organization: T::OrgId, who: T::AccountId, shares: T::Shares) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for issuance (the supervisor or the module's sudo account)
            let authentication: bool = Self::is_organization_supervisor(organization, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToIssueShares);

            Self::issue(organization, who.clone(), shares, false)?;
            Self::deposit_event(RawEvent::SharesIssued(organization, who, shares));
            Ok(())
        }
        #[weight = 0]
        fn burn_shares(origin, organization: T::OrgId, who: T::AccountId, shares: T::Shares) -> DispatchResult {
            let burner = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToBurnShares);
            // second check is that this is an authorized party for burning
            let authentication: bool = Self::is_organization_supervisor(organization, &burner);
            ensure!(authentication, Error::<T>::NotAuthorizedToBurnShares);

            Self::burn(organization, who.clone(), Some(shares), false)?;
            Self::deposit_event(RawEvent::SharesBurned(organization, who, shares));
            Ok(())
        }
        #[weight = 0]
        fn batch_issue_shares(origin, organization: T::OrgId, new_accounts: Vec<(T::AccountId, T::Shares)>) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for issuance
            let authentication: bool = Self::is_organization_supervisor(organization, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToIssueShares);
            let genesis: WeightedVector<T::AccountId, T::Shares> = new_accounts.into();
            let total_new_shares_minted = genesis.total();
            Self::batch_issue(organization, genesis)?;
            Self::deposit_event(RawEvent::SharesBatchIssued(organization, total_new_shares_minted));
            Ok(())
        }
        #[weight = 0]
        fn batch_burn_shares(origin, organization: T::OrgId, old_accounts: Vec<(T::AccountId, T::Shares)>) -> DispatchResult {
            let issuer = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToIssueShares);
            // second check is that this is an authorized party for burning
            let authentication: bool = Self::is_organization_supervisor(organization, &issuer);
            ensure!(authentication, Error::<T>::NotAuthorizedToBurnShares);
            let genesis: WeightedVector<T::AccountId, T::Shares> = old_accounts.into();
            let total_new_shares_burned = genesis.total();
            Self::batch_burn(organization, genesis)?;
            Self::deposit_event(RawEvent::SharesBatchBurned(organization, total_new_shares_burned));
            Ok(())
        }
        #[weight = 0]
        fn lock_shares(origin, organization: T::OrgId, who: T::AccountId) -> DispatchResult {
            let locker = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToLockShares);
            // second check is that this is an authorized party for locking shares
            let authentication: bool = Self::is_organization_supervisor(organization, &locker)
                                    || locker == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToLockShares);

            Self::lock_profile(organization, &who)?;
            Self::deposit_event(RawEvent::SharesLocked(organization, who));
            Ok(())
        }
        #[weight = 0]
        fn unlock_shares(origin, organization: T::OrgId, who: T::AccountId) -> DispatchResult {
            let unlocker = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToUnLockShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::is_organization_supervisor(organization, &unlocker)
                                    || unlocker == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToUnLockShares);

            Self::unlock_profile(organization, &who)?;
            Self::deposit_event(RawEvent::SharesUnlocked(organization, who));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_immediate_child(parent: T::OrgId, child: T::OrgId) -> bool {
        <OrgHierarchy<T>>::get(parent, child).is_some()
    }
    pub fn get_immediate_children(parent: T::OrgId) -> Option<Vec<T::OrgId>> {
        let ret = <OrgHierarchy<T>>::iter_prefix(parent)
            .map(|(id, _)| id)
            .collect::<Vec<T::OrgId>>();
        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
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

impl<T: Trait> OrganizationSupervisorPermissions<T::OrgId, T::AccountId>
    for Module<T>
{
    fn is_organization_supervisor(org: T::OrgId, who: &T::AccountId) -> bool {
        if let Some(state) = <OrganizationStates<T>>::get(org) {
            return state.is_sudo(who)
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
    fn put_organization_supervisor(
        org: T::OrgId,
        who: T::AccountId,
    ) -> DispatchResult {
        let old_org = <OrganizationStates<T>>::get(org)
            .ok_or(Error::<T>::OrganizationMustExistToPutSupervisor)?;
        let new_org = old_org.put_sudo(who);
        <OrganizationStates<T>>::insert(org, new_org);
        Ok(())
    }
}

impl<T: Trait> RegisterOrganization<T::OrgId, T::AccountId, T::Cid>
    for Module<T>
{
    type OrgSrc = OrganizationSource<T::AccountId, T::Shares>;
    type OrganizationState = Organization<T::AccountId, T::OrgId, T::Cid>;
    fn organization_from_src(
        src: Self::OrgSrc,
        org_id: T::OrgId,
        supervisor: Option<T::AccountId>,
        value_constitution: T::Cid,
    ) -> Result<Self::OrganizationState, DispatchError> {
        match src {
            OrganizationSource::Accounts(accounts) => {
                // batch_add (flat membership group)
                let weighted_acc = accounts
                    .into_iter()
                    .map(|acc| (acc, 1u32.into()))
                    .collect::<Vec<(T::AccountId, T::Shares)>>();
                Self::batch_issue(org_id, weighted_acc.into())?;
                Ok(Organization::new(supervisor, org_id, value_constitution))
            }
            OrganizationSource::AccountsWeighted(weighted_accounts) => {
                // batch_issue (share weighted membership group)
                Self::batch_issue(org_id, weighted_accounts.into())?;
                Ok(Organization::new(supervisor, org_id, value_constitution))
            }
        }
    }
    fn register_organization(
        source: Self::OrgSrc,
        supervisor: Option<T::AccountId>,
        value_constitution: T::Cid,
    ) -> Result<T::OrgId, DispatchError> {
        let new_org_id = Self::generate_unique_id();
        let new_organization = Self::organization_from_src(
            source,
            new_org_id,
            supervisor,
            value_constitution,
        )?;
        let new_org_count = <OrganizationCounter>::get() + 1u32;
        <OrganizationStates<T>>::insert(new_org_id, new_organization);
        <OrganizationCounter>::put(new_org_count);
        Ok(new_org_id)
    }
    fn register_sub_organization(
        parent_id: T::OrgId,
        source: Self::OrgSrc,
        supervisor: Option<T::AccountId>,
        value_constitution: T::Cid,
    ) -> Result<T::OrgId, DispatchError> {
        let new_org_id = Self::generate_unique_id();
        let new_organization = Self::organization_from_src(
            source,
            new_org_id,
            supervisor,
            value_constitution,
        )?;
        <OrgHierarchy<T>>::insert(parent_id, new_org_id, ());
        <OrganizationStates<T>>::insert(new_org_id, new_organization);
        let new_org_count = <OrganizationCounter>::get() + 1u32;
        <OrganizationCounter>::put(new_org_count);
        Ok(new_org_id)
    }
}
impl<T: Trait> RemoveOrganization<T::OrgId> for Module<T> {
    fn remove_organization(id: T::OrgId) -> DispatchResult {
        ensure!(
            !Self::id_is_available(id),
            Error::<T>::OrganizationCannotBeRemovedIfInputIdIsAvailable
        );
        <OrganizationStates<T>>::remove(id);
        let new_org_count = <OrganizationCounter>::get().saturating_sub(1u32);
        <OrganizationCounter>::put(new_org_count);
        Ok(())
    }
    fn recursive_remove_organization(id: T::OrgId) -> DispatchResult {
        ensure!(
            !Self::id_is_available(id),
            Error::<T>::OrganizationCannotBeRemovedIfInputIdIsAvailable
        );
        <OrgHierarchy<T>>::iter_prefix(id)
            .map(|(child, _)| -> DispatchResult {
                Self::recursive_remove_organization(child)
            })
            .collect::<DispatchResult>()?;
        Self::remove_organization(id)?;
        Self::deposit_event(RawEvent::OrganizationRemoved(id));
        Ok(())
    }
}

impl<T: Trait> GetGroup<T::OrgId, T::AccountId> for Module<T> {
    fn get_group(organization: T::OrgId) -> Option<OrderedSet<T::AccountId>> {
        if !Self::id_is_available(organization) {
            Some(
                // is this the same performance as a get() with Key=Vec<AccountId>
                <Members<T>>::iter()
                    .filter(|(org, _, _)| *org == organization)
                    .map(|(_, account, _)| account)
                    .collect::<Vec<_>>()
                    .into(),
            )
        } else {
            None
        }
    }
}

impl<T: Trait> ShareInformation<T::OrgId, T::AccountId, T::Shares>
    for Module<T>
{
    type Profile = ShareProfile<T::Shares, ProfileState>;
    type Genesis = WeightedVector<T::AccountId, T::Shares>;
    /// Gets the total number of shares issued for an organization's share identifier
    fn outstanding_shares(organization: T::OrgId) -> T::Shares {
        <TotalIssuance<T>>::get(organization)
    }
    /// Get input's share profile if it exists
    fn get_share_profile(
        organization: T::OrgId,
        who: &T::AccountId,
    ) -> Option<Self::Profile> {
        <Members<T>>::get(organization, who)
    }
    /// Returns the entire membership group associated with a share identifier, fallible bc checks existence
    fn get_membership_with_shape(
        organization: T::OrgId,
    ) -> Option<Self::Genesis> {
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
    type Proportion = SharePortion<T::Shares, Permill>;
    fn issue(
        organization: T::OrgId,
        new_owner: T::AccountId,
        amount: T::Shares,
        batch: bool,
    ) -> DispatchResult {
        let new_profile = if let Some(existing_profile) =
            <Members<T>>::get(organization, &new_owner)
        {
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
    fn burn(
        organization: T::OrgId,
        old_owner: T::AccountId,
        amount: Option<T::Shares>,
        batch: bool,
    ) -> Result<Self::Proportion, DispatchError> {
        let old_profile = <Members<T>>::get(organization, &old_owner)
            .ok_or(Error::<T>::NotEnoughSharesToSatisfyBurnRequest)?;
        let old_issuance = <TotalIssuance<T>>::get(organization);
        let amt_to_burn = if let Some(specific_amt) = amount {
            ensure!(
                old_profile.total() >= specific_amt,
                Error::<T>::NotEnoughSharesToSatisfyBurnRequest
            );
            specific_amt
        } else {
            old_profile.total()
        };
        let portion =
            Permill::from_rational_approximation(amt_to_burn, old_issuance);
        ensure!(
            old_issuance >= amt_to_burn,
            Error::<T>::CannotBurnMoreThanTotalIssuance
        );
        if !batch {
            let new_issuance = old_issuance - amt_to_burn;
            <TotalIssuance<T>>::insert(organization, new_issuance);
        }
        let new_profile = old_profile.subtract_shares(amt_to_burn);
        if new_profile.is_zero() {
            // leave the group
            <Members<T>>::remove(organization, old_owner);
        } else {
            <Members<T>>::insert(organization, old_owner, new_profile);
        }
        Ok(SharePortion::new(amt_to_burn, portion))
    }
    fn batch_issue(
        organization: T::OrgId,
        genesis: Self::Genesis,
    ) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance = <TotalIssuance<T>>::get(organization);
        let new_issuance = old_issuance
            .checked_add(&genesis.total())
            .ok_or(Error::<T>::IssuanceWouldOverflowShares)?;
        genesis.vec().into_iter().for_each(|(member, shares)| {
            if let Ok(()) =
                Self::issue(organization, member.clone(), shares, true)
            {
                Self::deposit_event(RawEvent::SharesIssued(
                    organization,
                    member,
                    shares,
                ));
            }
        });
        <TotalIssuance<T>>::insert(organization, new_issuance);
        Ok(())
    }
    fn batch_burn(
        organization: T::OrgId,
        genesis: Self::Genesis,
    ) -> DispatchResult {
        ensure!(
            genesis.verify_shape(),
            Error::<T>::GenesisTotalMustEqualSumToUseBatchOps
        );
        let old_issuance = <TotalIssuance<T>>::get(organization);
        let new_issuance = old_issuance
            .checked_sub(&genesis.total())
            .ok_or(Error::<T>::IssuanceCannotGoNegative)?;
        genesis.vec().into_iter().for_each(|(member, shares)| {
            if let Ok(portion) =
                Self::burn(organization, member.clone(), Some(shares), true)
            {
                Self::deposit_event(RawEvent::SharesBurned(
                    organization,
                    member,
                    portion.total(),
                ));
            }
        });
        <TotalIssuance<T>>::insert(organization, new_issuance);
        Ok(())
    }
}
impl<T: Trait> LockProfile<T::OrgId, T::AccountId> for Module<T> {
    fn lock_profile(
        organization: T::OrgId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let old_profile = <Members<T>>::get(organization, who)
            .ok_or(Error::<T>::CannotLockProfileThatDNE)?;
        ensure!(
            old_profile.is_unlocked(),
            Error::<T>::CannotLockIfAlreadyLocked
        );
        let new_profile = old_profile.lock();
        <Members<T>>::insert(organization, who, new_profile);
        Ok(())
    }
    fn unlock_profile(
        organization: T::OrgId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let old_profile = <Members<T>>::get(organization, who)
            .ok_or(Error::<T>::CannotUnLockProfileThatDNE)?;
        ensure!(
            !old_profile.is_unlocked(),
            Error::<T>::CannotUnLockIfAlreadyUnLocked
        );
        let new_profile = old_profile.unlock();
        <Members<T>>::insert(organization, who, new_profile);
        Ok(())
    }
}
