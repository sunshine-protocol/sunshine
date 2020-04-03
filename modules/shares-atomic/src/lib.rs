#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)] // noted that I have lots of generics...
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::{
    share::{AtomicShareProfile, SimpleShareGenesis},
    traits::{
        GenerateUniqueID, GetProfile, GroupMembership, IDIsAvailable, LockableProfile,
        ReservableProfile, ShareBank, ShareRegistration, VerifyShape,
    },
    uuid::OrgSharePrefixKey,
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};

pub trait Trait<I = DefaultInstance>: system::Trait {
    /// The overarching event type
    type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

    /// The identifier for an organization
    type OrgId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    /// The share identifier type
    type ShareId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    /// The share type
    type Share: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    /// The hard limit on the number of times shares can be reserved
    type ReservationLimit: Get<u32>; // TODO: add softer limit that can be governed by supervisors
}

decl_event!(
    pub enum Event<T, I=DefaultInstance>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait<I>>::OrgId,
        <T as Trait<I>>::Share,
        <T as Trait<I>>::ShareId,
    {
        /// Organization ID, Share Id, Account ID of reservee, times_reserved of their profile
        SharesReserved(OrgId, ShareId, AccountId, u32),
        /// Organization ID, Share Id, Account ID of unreservee, times_reserved of their profile
        SharesUnReserved(OrgId, ShareId, AccountId, u32),
        /// Organization ID, Share Id, Account Id
        SharesLocked(OrgId, ShareId, AccountId),
        /// Organization ID, Share Id, Account Id
        SharesUnlocked(OrgId, ShareId, AccountId),
        /// Organization ID, Share Id
        NewShareType(OrgId, ShareId),
        /// Organization ID, Share Id, Recipient AccountId, Issued Amount
        Issuance(OrgId, ShareId, AccountId, Share),
        /// Organization ID, Share Id, Burned AccountId, Burned Amount
        Burn(OrgId, ShareId, AccountId, Share),
        /// Organization IDm Share Id, All Shares in Circulation
        TotalSharesIssued(OrgId, ShareId, Share),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait<I>, I: Instance> {
        ReservationWouldExceedHardLimit,
        CannotUnreserveWithZeroReservations,
        ShareTypeNotRegistered,
        ShareHolderMembershipUninitialized,
        ProfileNotInstantiated,
        CanOnlyBurnReservedShares,
        IssuanceCannotGoNegative,
        CannotIssueToLockedProfile,
        InitialIssuanceShapeIsInvalid,
        CantReserveMoreThanShareTotal,
    }
}

decl_storage! {
    trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Shares {
        pub ShareIdCounter get(share_id_counter):
            map hasher(blake2_256) T::OrgId => T::ShareId;

        /// Total share issuance for the share type with `ShareId`
        /// also the main point of registration for (OrgId, ShareId) pairs (see `GenerateUniqueId`)
        pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T, I>| {
            config.total_issuance.clone()
        }): double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) T::ShareId => Option<T::Share>;

        /// The total shareholders; must be kept in sync with `Profile` and `TotalIssuance`
        pub ShareHolders get(fn share_holders) build(|config: &GenesisConfig<T, I>| {
            config.shareholder_membership.clone()
        }): double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) T::ShareId => Option<Vec<T::AccountId>>;

        /// The ShareProfile (use module type once `StoredMap` is added in #4820)
        pub Profile get(fn profile) build(|config: &GenesisConfig<T, I>| {
            config.membership_shares.iter().map(|(org, id, who, shares)| {
                let share_profile = AtomicShareProfile::new_shares(*shares);
                let org_share_id = OrgSharePrefixKey::new(*org, *id);
                (org_share_id, who.clone(), share_profile)
            }).collect::<Vec<_>>()
        }): double_map hasher(blake2_256) OrgSharePrefixKey<T::OrgId, T::ShareId>, hasher(blake2_256) T::AccountId => Option<AtomicShareProfile<T::Share>>;
    }
    add_extra_genesis {
        config(membership_shares): Vec<(T::OrgId, T::ShareId, T::AccountId, T::Share)>;
        // REQUIRED: no duplicate share_id entries in this vector; membership_shares.sum() == total_issuance
        config(total_issuance): Vec<(T::OrgId, T::ShareId, T::Share)>;
        // REQUIRED: syncs with membership_shares on membership organization
        config(shareholder_membership): Vec<(T::OrgId, T::ShareId, Vec<T::AccountId>)>;
    }
}

decl_module! {
    pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
        type Error = Error<T, I>;
        fn deposit_event() = default;

        const ReservationLimit: u32 = T::ReservationLimit::get();

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn register_shares(origin, organization: T::OrgId, share_id: T::ShareId, genesis: Vec<(T::AccountId, T::Share)>) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let assigned_share_id = Self::register(organization, share_id, genesis.into())?;
            Self::deposit_event(RawEvent::NewShareType(organization, assigned_share_id));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn lock_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::lock_profile(organization, share_id, &who)?;
            Self::deposit_event(RawEvent::SharesLocked(organization, share_id, who));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn unlock_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::unlock_profile(organization, share_id, &who)?;
            Self::deposit_event(RawEvent::SharesUnlocked(organization, share_id, who));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn reserve_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let reservation_context = Self::reserve(organization, share_id, &who, None)?;
            let times_reserved = reservation_context.0;
            Self::deposit_event(RawEvent::SharesReserved(organization, share_id, who, times_reserved));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn unreserve_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            let reservation_context = Self::unreserve(organization, share_id, &who, None)?;
            let times_reserved = reservation_context.0;
            Self::deposit_event(RawEvent::SharesUnReserved(organization, share_id, who, times_reserved));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn issue_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId, shares: T::Share) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::issue(organization, share_id, &who, shares)?;
            Self::deposit_event(RawEvent::Issuance(organization, share_id, who, shares));
            Ok(())
        }

        // WARNING
        // access needs to be permissioned, never callable in production by anyone
        fn burn_shares(origin, organization: T::OrgId, share_id: T::ShareId, who: T::AccountId, shares: T::Share) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::burn(organization, share_id, &who, shares)?;
            Self::deposit_event(RawEvent::Burn(organization, share_id, who, shares));
            Ok(())
        }
    }
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
    /// Set the ShareProfile
    fn set_profile(
        prefix_key: OrgSharePrefixKey<T::OrgId, T::ShareId>,
        who: &T::AccountId,
        new: &AtomicShareProfile<T::Share>,
    ) -> DispatchResult {
        Profile::<T, I>::insert(prefix_key, who, new);
        Ok(())
    }
}

impl<T: Trait<I>, I: Instance> IDIsAvailable<OrgSharePrefixKey<T::OrgId, T::ShareId>>
    for Module<T, I>
{
    fn id_is_available(id: OrgSharePrefixKey<T::OrgId, T::ShareId>) -> bool {
        let organization = id.org();
        let share_id = id.share();
        None == <TotalIssuance<T, I>>::get(organization, share_id)
    }
}

impl<T: Trait<I>, I: Instance> GenerateUniqueID<OrgSharePrefixKey<T::OrgId, T::ShareId>>
    for Module<T, I>
{
    fn generate_unique_id(
        proposed_id: OrgSharePrefixKey<T::OrgId, T::ShareId>,
    ) -> OrgSharePrefixKey<T::OrgId, T::ShareId> {
        if !Self::id_is_available(proposed_id) {
            let organization = proposed_id.org();
            let mut id_counter = <ShareIdCounter<T, I>>::get(organization);
            while <TotalIssuance<T, I>>::get(organization, id_counter).is_some() {
                // TODO: add overflow check here
                id_counter += 1.into();
            }
            <ShareIdCounter<T, I>>::insert(organization, id_counter + 1.into());
            OrgSharePrefixKey::new(organization, id_counter)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait<I>, I: Instance> GroupMembership<T::AccountId> for Module<T, I> {
    type GroupId = OrgSharePrefixKey<T::OrgId, T::ShareId>;

    // constant time membership check for balanced tries ;)
    fn is_member_of_group(group_id: Self::GroupId, who: &T::AccountId) -> bool {
        <Profile<T, I>>::get(group_id, who).is_some()
    }
}

impl<T: Trait<I>, I: Instance> ShareRegistration<T::AccountId> for Module<T, I> {
    // provide access to these types from other modules
    type OrgId = T::OrgId;
    type ShareId = T::ShareId;
    type Shares = T::Share;
    type GenesisAllocation = SimpleShareGenesis<T::AccountId, T::Share>;

    /// This registration logic is relatively expensive and has complexity that is linear in the size of the group
    fn register(
        organization: T::OrgId,
        proposed_id: Self::ShareId,
        issuance: Self::GenesisAllocation,
    ) -> Result<Self::ShareId, DispatchError> {
        let proposed_joint_id = OrgSharePrefixKey::new(organization, proposed_id);
        let org_share_id = Self::generate_unique_id(proposed_joint_id);
        let share_id = org_share_id.share();
        // TODO: add registration conditions specific to every share_id which might include belonging to an existing share_id
        ensure!(
            issuance.verify_shape(),
            Error::<T, I>::InitialIssuanceShapeIsInvalid
        );
        let mut shareholders: Vec<T::AccountId> = Vec::new();
        for account_share in issuance.account_ownership.iter() {
            let new_profile = AtomicShareProfile::new_shares(account_share.1);
            shareholders.push(account_share.0.clone());
            let prefix_key = OrgSharePrefixKey::new(organization, share_id);
            Profile::<T, I>::insert(prefix_key, account_share.0.clone(), new_profile);
        }
        // because I'm not using `issue` above and inserting the profile directly, I call issuance directly and separately
        <TotalIssuance<T, I>>::insert(organization, share_id, issuance.total);
        <ShareHolders<T, I>>::insert(organization, share_id, shareholders);
        Ok(share_id)
    }
}

impl<T: Trait<I>, I: Instance> ReservableProfile<T::AccountId> for Module<T, I> {
    type ReservationContext = (u32, Self::Shares);

    fn reserve(
        organization: T::OrgId,
        share_id: Self::ShareId,
        who: &T::AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError> {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let fetched_profile = Profile::<T, I>::get(prefix_key, who);
        ensure!(
            fetched_profile.is_some(),
            Error::<T, I>::ProfileNotInstantiated
        );
        let old_profile = fetched_profile.expect("checked existence of inner value in line above");
        // TODO: allow multiple reservations for a single vote? this should be a layered config option
        let max_amount_is_share_total = old_profile.get_shares();
        // THIS DESIGN ACCEPTS ANY RESERVE/UNRESERVE AS INCREMENTING/DECREMENTING TIMES_RESERVED IFF LESS_THAN TOTAL
        // TODO: if amt.0 > 1, iterate times_reserved by the number of times required; for now, we assume amt.0 == 1
        let amount_or_default = if let Some(amt) = amount {
            amt.1
        } else {
            max_amount_is_share_total
        };
        ensure!(
            max_amount_is_share_total >= amount_or_default,
            Error::<T, I>::CantReserveMoreThanShareTotal
        );
        let times_reserved = old_profile.get_times_reserved() + 1u32;
        // make sure it's below the hard reservation limit
        ensure!(
            times_reserved < T::ReservationLimit::get(),
            Error::<T, I>::ReservationWouldExceedHardLimit
        );
        // instantiate new share profile which just iterates times_reserved
        let new_share_profile = old_profile.iterate_times_reserved(1u32);
        // set new share profile for `who`
        let new_times_reserved = new_share_profile.get_times_reserved();
        Self::set_profile(prefix_key, who, &new_share_profile)?;
        Ok((new_times_reserved, amount_or_default))
    }

    fn unreserve(
        organization: T::OrgId,
        share_id: Self::ShareId,
        who: &T::AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError> {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let fetched_profile = Profile::<T, I>::get(prefix_key, who);
        ensure!(
            fetched_profile.is_some(),
            Error::<T, I>::ProfileNotInstantiated
        );
        let old_profile = fetched_profile.expect("checked existence of inner value in line above");
        // TODO: allow multiple reservations for a single vote? this should be a layered config option
        let max_amount_is_share_total = old_profile.get_shares();
        // THIS DESIGN ACCEPTS ANY RESERVE/UNRESERVE AS INCREMENTING/DECREMENTING TIMES_RESERVED IFF LESS_THAN TOTAL
        // TODO: if amt.0 > 1, iterate times_reserved by the number of times required; for now, we assume amt.0 == 1
        let amount_or_default = if let Some(amt) = amount {
            amt.1
        } else {
            max_amount_is_share_total
        };
        ensure!(
            max_amount_is_share_total >= amount_or_default,
            Error::<T, I>::CantReserveMoreThanShareTotal
        );
        let current_times_reserved = old_profile.get_times_reserved();
        ensure!(
            current_times_reserved >= 1u32,
            Error::<T, I>::CannotUnreserveWithZeroReservations
        );
        // instantiate new share profile by incrementing times reserved
        let new_share_profile = old_profile.decrement_times_reserved(1u32);
        // set new share profile
        let new_times_reserved = new_share_profile.get_times_reserved();
        Self::set_profile(prefix_key, who, &new_share_profile)?;
        Ok((new_times_reserved, amount_or_default))
    }
}

impl<T: Trait<I>, I: Instance> LockableProfile<T::AccountId> for Module<T, I> {
    fn lock_profile(
        organization: T::OrgId,
        share_id: Self::ShareId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let profile = Profile::<T, I>::get(prefix_key, who);
        let locked_profile = if let Some(to_be_locked) = profile {
            to_be_locked.lock()
        } else {
            return Err(Error::<T, I>::ProfileNotInstantiated.into());
        };
        // lock the profile
        Profile::<T, I>::insert(prefix_key, who, locked_profile);
        Ok(())
    }

    fn unlock_profile(
        organization: T::OrgId,
        share_id: Self::ShareId,
        who: &T::AccountId,
    ) -> DispatchResult {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let profile = Profile::<T, I>::get(prefix_key, who);
        let locked_profile = if let Some(to_be_locked) = profile {
            to_be_locked.unlock()
        } else {
            return Err(Error::<T, I>::ProfileNotInstantiated.into());
        };
        // lock the profile
        Profile::<T, I>::insert(prefix_key, who, locked_profile);
        Ok(())
    }
}

impl<T: Trait<I>, I: Instance> ShareBank<T::AccountId> for Module<T, I> {
    fn outstanding_shares(organization: T::OrgId, share_id: Self::ShareId) -> T::Share {
        if let Some(amount) = <TotalIssuance<T, I>>::get(organization, share_id) {
            amount
        } else {
            0.into()
        }
    }

    fn shareholder_membership(
        organization: T::OrgId,
        share_id: Self::ShareId,
    ) -> Result<Vec<T::AccountId>, DispatchError> {
        if let Some(membership) = <ShareHolders<T, I>>::get(organization, share_id) {
            Ok(membership)
        } else {
            Err(Error::<T, I>::ShareHolderMembershipUninitialized.into())
        }
    }

    fn issue(
        organization: T::OrgId,
        share_id: Self::ShareId,
        new_owner: &T::AccountId,
        amount: Self::Shares,
    ) -> DispatchResult {
        let current_issuance = <TotalIssuance<T, I>>::get(organization, share_id)
            .ok_or(Error::<T, I>::ShareTypeNotRegistered)?;
        // update total issuance
        let new_amount = current_issuance + amount;
        <TotalIssuance<T, I>>::insert(organization, share_id, new_amount);
        // update the recipient's share profile
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let old_share_profile = Profile::<T, I>::get(prefix_key, new_owner);
        let new_share_profile = if let Some(old_profile) = old_share_profile {
            // TODO: checked_add
            ensure!(
                old_profile.is_unlocked(),
                Error::<T, I>::CannotIssueToLockedProfile
            );
            old_profile.add_shares(amount)
        } else {
            // new share profile, could place a check here for membership in conditional other shares in the organization?
            AtomicShareProfile::new_shares(amount)
        };
        Profile::<T, I>::insert(prefix_key, &new_owner, new_share_profile);
        Ok(())
    }

    fn burn(
        organization: T::OrgId,
        share_id: Self::ShareId,
        old_owner: &T::AccountId,
        amount: Self::Shares,
    ) -> DispatchResult {
        // (1) change total issuance
        let current_issuance = <TotalIssuance<T, I>>::get(organization, share_id)
            .ok_or(Error::<T, I>::ShareTypeNotRegistered)?;
        // (2) change owner's profile
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let profile = Profile::<T, I>::get(prefix_key, old_owner)
            .ok_or(Error::<T, I>::ProfileNotInstantiated)?;
        // enforce invariant that the owner must have these shares to burn them
        let total_shares = &profile.get_shares();
        ensure!(
            total_shares >= &amount,
            Error::<T, I>::CanOnlyBurnReservedShares
        );
        // ..(1)
        ensure!(
            current_issuance >= amount,
            Error::<T, I>::IssuanceCannotGoNegative
        );
        let new_amount = current_issuance - amount;
        <TotalIssuance<T, I>>::insert(organization, share_id, new_amount);
        // ..(2)
        let new_profile = profile.subtract_shares(amount);
        Profile::<T, I>::insert(prefix_key, old_owner, new_profile);
        Ok(())
    }
}

// For testing purposes only
impl<T: Trait<I>, I: Instance> GetProfile<T::AccountId> for Module<T, I> {
    fn get_share_profile(
        organization: T::OrgId,
        share_id: Self::ShareId,
        who: &T::AccountId,
    ) -> Result<Self::Shares, DispatchError> {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        let wrapped_profile = Profile::<T, I>::get(prefix_key, who);
        if let Some(profile) = wrapped_profile {
            Ok(profile.get_shares())
        } else {
            Err(Error::<T, I>::ProfileNotInstantiated.into())
        }
    }
}
