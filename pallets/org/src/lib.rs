#![recursion_limit = "256"]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This module expresses a framework for configurable group governance

#[cfg(test)]
mod tests;

use util::{
    organization::{
        Organization,
        OrganizationSource,
    },
    share::{
        ShareProfile,
        SimpleShareGenesis,
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
        ReserveProfile,
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
    storage::{
        IterableStorageDoubleMap,
        IterableStorageMap,
    },
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
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};

pub trait Trait: system::Trait {
    /// Overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid type
    type IpfsReference: Parameter + Member + Default;

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
        <T as Trait>::IpfsReference,
    {
        /// No shares issued but an organization was registered with flat membership with the last `u32` as the number of members
        NewFlatOrganizationRegistered(AccountId, OrgId, IpfsReference, u32),
        /// Shares issued for a weighted ownership org s.t. the last element `Shares` is total issuance
        NewWeightedOrganizationRegistered(AccountId, OrgId, IpfsReference, Shares),
        /// Organization ID, New Member Account ID, Amount Issued
        NewMemberAddedToOrg(OrgId, AccountId, Shares),
        /// Organization ID, Old Member Account Id, Amount Burned
        OldMemberRemovedFromOrg(OrgId, AccountId, Shares),
        /// Batch Addition by the Account ID, _,  total shares added
        BatchMemberAdditionForOrg(AccountId, OrgId, Shares),
        /// Batch Removal by the Account ID, _, total shares burned
        BatchMemberRemovalForOrg(AccountId, OrgId, Shares),
        /// Organization ID, Account ID of reservee, times_reserved of their profile
        SharesReserved(OrgId, AccountId, Shares),
        /// Organization ID, Account ID of unreservee, times_reserved of their profile
        SharesUnReserved(OrgId, AccountId, Shares),
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
        CannotReserveMoreThanShareTotal,
        CannotUnReserveMoreThanShareTotal,
        CannotLockIfAlreadyLocked,
        CannotUnLockIfAlreadyUnLocked,
        CannotUnLockProfileThatDNE,
        CannotLockProfileThatDNE,
        CannotReserveIfMemberProfileDNE,
        CannotUnReserveIfMemberProfileDNE,
        OrganizationMustBeRegisteredToIssueShares,
        OrganizationMustBeRegisteredToBurnShares,
        OrganizationMustBeRegisteredToLockShares,
        OrganizationMustBeRegisteredToUnLockShares,
        OrganizationMustBeRegisteredToReserveShares,
        OrganizationMustBeRegisteredToUnReserveShares,
        NotAuthorizedToLockShares,
        NotAuthorizedToUnLockShares,
        NotAuthorizedToReserveShares,
        NotAuthorizedToUnReserveShares,
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
            hasher(blake2_128_concat) T::OrgId => Option<Organization<T::AccountId, T::OrgId, T::IpfsReference>>;

        /// The map to track organizational membership
        pub Members get(fn members): double_map
            hasher(blake2_128_concat) T::OrgId,
            hasher(blake2_128_concat) T::AccountId => Option<ShareProfile<T::Shares>>;

        /// Total number of outstanding shares that express relative ownership in group
        pub TotalIssuance get(fn total_issuance): map
            hasher(opaque_blake2_256) T::OrgId => T::Shares;
    }
    add_extra_genesis {
        config(first_organization_supervisor): T::AccountId;
        config(first_organization_value_constitution): T::IpfsReference;
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
            constitution: T::IpfsReference,
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
            constitution: T::IpfsReference,
            weighted_members: Vec<(T::AccountId, T::Shares)>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // auth will usually be specific to the module context in which this is used
            let wm_cpy: SimpleShareGenesis<T::AccountId, T::Shares> = weighted_members.clone().into();
            let new_id = if let Some(parent_id) = parent_org {
                Self::register_sub_organization(parent_id, OrganizationSource::AccountsWeighted(weighted_members), sudo, constitution.clone())?
            } else {
                Self::register_organization(OrganizationSource::AccountsWeighted(weighted_members), sudo, constitution.clone())?
            };
            Self::deposit_event(RawEvent::NewWeightedOrganizationRegistered(caller, new_id, constitution, wm_cpy.total()));
            Ok(())
        }
        /// Share Issuance Runtime Methods
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
            let genesis: SimpleShareGenesis<T::AccountId, T::Shares> = new_accounts.into();
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
            let genesis: SimpleShareGenesis<T::AccountId, T::Shares> = old_accounts.into();
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
        #[weight = 0]
        fn reserve_shares(origin, organization: T::OrgId, who: T::AccountId) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToReserveShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::is_organization_supervisor(organization, &reserver)
                                    || reserver == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToReserveShares);


            let amount_reserved = Self::reserve(organization, &who, None)?;
            Self::deposit_event(RawEvent::SharesReserved(organization, who, amount_reserved));
            Ok(())
        }
        #[weight = 0]
        fn unreserve_shares(origin, organization: T::OrgId, who: T::AccountId) -> DispatchResult {
            let unreserver = ensure_signed(origin)?;
            // first check is that the organization exists
            ensure!(!Self::id_is_available(organization), Error::<T>::OrganizationMustBeRegisteredToUnReserveShares);
            // second check is that this is an authorized party for unlocking shares
            let authentication: bool = Self::is_organization_supervisor(organization, &unreserver)
                                    || unreserver == who;
            ensure!(authentication, Error::<T>::NotAuthorizedToUnReserveShares);

            let amount_unreserved = Self::unreserve(organization, &who, None)?;
            Self::deposit_event(RawEvent::SharesUnReserved(organization, who, amount_unreserved));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_immediate_child(
        parent: Option<T::OrgId>,
        child: T::OrgId,
    ) -> Result<bool, DispatchError> {
        let child_org =
            <OrganizationStates<T>>::get(child).ok_or(Error::<T>::OrgDNE)?;
        Ok(child_org.parent() == parent)
    }
    pub fn get_immediate_children(parent: T::OrgId) -> Option<Vec<T::OrgId>> {
        let ret = <OrganizationStates<T>>::iter()
            .filter(|(_, org)| org.parent() == Some(parent))
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

impl<T: Trait> RegisterOrganization<T::OrgId, T::AccountId, T::IpfsReference>
    for Module<T>
{
    type OrgSrc = OrganizationSource<T::AccountId, T::Shares>;
    type OrganizationState =
        Organization<T::AccountId, T::OrgId, T::IpfsReference>;
    fn organization_from_src(
        src: Self::OrgSrc,
        org_id: T::OrgId,
        parent_id: Option<T::OrgId>,
        supervisor: Option<T::AccountId>,
        value_constitution: T::IpfsReference,
    ) -> Result<Self::OrganizationState, DispatchError> {
        match src {
            OrganizationSource::Accounts(accounts) => {
                // batch_add (flat membership group)
                let weighted_acc = accounts
                    .into_iter()
                    .map(|acc| (acc, 1u32.into()))
                    .collect::<Vec<(T::AccountId, T::Shares)>>();
                Self::batch_issue(org_id, weighted_acc.into())?;
                Ok(Organization::new(supervisor, parent_id, value_constitution))
            }
            OrganizationSource::AccountsWeighted(weighted_accounts) => {
                // batch_issue (share weighted membership group)
                Self::batch_issue(org_id, weighted_accounts.into())?;
                Ok(Organization::new(supervisor, parent_id, value_constitution))
            }
        }
    }
    fn register_organization(
        source: Self::OrgSrc,
        supervisor: Option<T::AccountId>,
        value_constitution: T::IpfsReference,
    ) -> Result<T::OrgId, DispatchError> {
        let new_org_id = Self::generate_unique_id();
        let new_organization = Self::organization_from_src(
            source,
            new_org_id,
            None,
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
        value_constitution: T::IpfsReference,
    ) -> Result<T::OrgId, DispatchError> {
        let new_org_id = Self::generate_unique_id();
        // TODO: bound depth instead of current unbounded size
        let new_organization = Self::organization_from_src(
            source,
            new_org_id,
            Some(parent_id),
            supervisor,
            value_constitution,
        )?;
        let new_org_count = <OrganizationCounter>::get() + 1u32;
        <OrganizationStates<T>>::insert(new_org_id, new_organization);
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
        <OrganizationStates<T>>::iter()
            .filter(|(_, org)| (*org).parent() == Some(id))
            .map(|(child_id, _)| -> DispatchResult {
                <OrganizationStates<T>>::remove(child_id);
                let new_org_count =
                    <OrganizationCounter>::get().saturating_sub(1u32);
                <OrganizationCounter>::put(new_org_count);
                Self::recursive_remove_organization(child_id)
            })
            .collect::<DispatchResult>()
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
    type Profile = ShareProfile<T::Shares>;
    type Genesis = SimpleShareGenesis<T::AccountId, T::Shares>;
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
    ) -> DispatchResult {
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
        Ok(())
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
        genesis
            .account_ownership()
            .into_iter()
            .map(|(member, shares)| -> DispatchResult {
                Self::burn(organization, member, Some(shares), true)
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
