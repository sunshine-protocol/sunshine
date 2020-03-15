#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::{
    share::{ShareProfile, SimpleShareGenesis},
    traits::{
        GenerateUniqueID, GetProfile, IDIsAvailable, ReservableProfile, ShareBank,
        ShareRegistration, VerifyShape,
    },
};

use codec::Codec;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};

pub trait Trait<I = DefaultInstance>: system::Trait {
    /// The overarching event type
    type Event: From<Event<Self, I>> + Into<<Self as frame_system::Trait>::Event>;

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

    /// The share identifier type
    type ShareId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;
}

decl_event!(
    pub enum Event<T, I=DefaultInstance>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait<I>>::Share,
        <T as Trait<I>>::ShareId,
    {
        /// Reserver ID, Share Type, Amount
        SharesReserved(AccountId, ShareId, Share),
        /// UnReserver ID, Share Type, Amount
        SharesUnReserved(AccountId, ShareId, Share),
        /// Registrar ID, New Share Type ID
        NewShareType(AccountId, ShareId),
        /// Recipient of Issuance Account, Share Id with New Issuance
        Issuance(AccountId, ShareId, Share),
        /// Burned Account, Share Id with New Burning (_buyback_)
        Buyback(AccountId, ShareId, Share),
        /// Share Id, All Shares Issued
        TotalSharesIssued(ShareId, Share),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait<I>, I: Instance> {
        CannotAffordToReserveShareBond,
        CannotAffordToUnreserveShareBond,
        ShareTypeNotRegistered,
        ShareHolderMembershipUninitialized,
        ProfileNotInstantiated,
        CanOnlyBurnReservedShares,
        InitialIssuanceShapeIsInvalid,
    }
}

decl_storage! {
    trait Store for Module<T: Trait<I>, I: Instance=DefaultInstance> as Shares {
        pub ShareIdCounter get(share_id_counter): T::ShareId;

        /// Total share issuance for the share type with `ShareId`
        pub TotalIssuance get(fn total_issuance) build(|config: &GenesisConfig<T, I>| {
            config.total_issuance.clone()
        }): map hasher(blake2_256) T::ShareId => Option<T::Share>;

        /// The total shareholders; must be kept in sync with `Profile` and `TotalIssuance`
        pub ShareHolders get(fn share_holders) build(|config: &GenesisConfig<T, I>| {
            config.shareholder_membership.clone()
        }): map hasher(blake2_256) T::ShareId => Option<Vec<T::AccountId>>;

        /// The ShareProfile (use module type once `StoredMap` is added in #4820)
        pub Profile get(fn profile) build(|config: &GenesisConfig<T, I>| {
            config.membership_shares.iter().map(|(who, id, shares)| {
                let share_profile = ShareProfile {
                    free: *shares,
                    ..Default::default()
                };
                (who.clone(), id.clone(), share_profile)
            }).collect::<Vec<_>>()
        }): double_map hasher(blake2_256) T::AccountId, hasher(blake2_256) T::ShareId => Option<ShareProfile<T::Share>>;
    }
    add_extra_genesis {
        // REQUIRED: membership_shares.sum() == total_issuance
        config(membership_shares): Vec<(T::AccountId, T::ShareId, T::Share)>;
        // REQUIRED: no duplicate share_id entries in this vector
        config(total_issuance): Vec<(T::ShareId, T::Share)>;
        // REQUIRED: syncs with membership_shares on membership organization
        config(shareholder_membership): Vec<(T::ShareId, Vec<T::AccountId>)>;
    }
}

decl_module! {
    pub struct Module<T: Trait<I>, I: Instance=DefaultInstance> for enum Call where origin: T::Origin {
        type Error = Error<T, I>;
        fn deposit_event() = default;

        // this is a very limited share genesis configuration
        fn register_shares(origin, proposed_share_id: T::ShareId, genesis: Vec<(T::AccountId, T::Share)>) -> DispatchResult {
            let registrar = ensure_signed(origin)?;
            let assigned_share_id = Self::register(proposed_share_id, genesis.into())?;
            Self::deposit_event(RawEvent::NewShareType(registrar, assigned_share_id));
            Ok(())
        }

        fn reserve_all_free_shares(origin, share_id: T::ShareId) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            let shares_reserved = Self::reserve(&reserver, share_id, None)?;
            Self::deposit_event(RawEvent::SharesReserved(reserver, share_id, shares_reserved));
            Ok(())
        }

        fn reserve_shares(origin, share_id: T::ShareId, shares: T::Share) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            let shares_reserved = Self::reserve(&reserver, share_id, Some(shares))?;
            Self::deposit_event(RawEvent::SharesReserved(reserver, share_id, shares_reserved));
            Ok(())
        }

        fn unreserve_all_shares(origin, share_id: T::ShareId) -> DispatchResult {
            let unreserver = ensure_signed(origin)?;
            let shares_unreserved = Self::unreserve(&unreserver, share_id, None)?;
            Self::deposit_event(RawEvent::SharesUnReserved(unreserver, share_id, shares_unreserved));
            Ok(())
        }

        fn unreserve_shares(origin, share_id: T::ShareId, shares: T::Share) -> DispatchResult {
            let unreserver = ensure_signed(origin)?;
            let shares_unreserved = Self::unreserve(&unreserver, share_id, Some(shares))?;
            Self::deposit_event(RawEvent::SharesUnReserved(unreserver, share_id, shares_unreserved));
            Ok(())
        }

        fn issue_shares(origin, owner: T::AccountId, share_id: T::ShareId, shares: T::Share) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::issue(&owner, share_id, shares.clone())?;
            Self::deposit_event(RawEvent::Issuance(owner, share_id, shares));
            Ok(())
        }

        fn buyback_shares(origin, owner: T::AccountId, share_id: T::ShareId, shares: T::Share) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Self::buyback(&owner, share_id, shares)?;
            Self::deposit_event(RawEvent::Buyback(owner, share_id, shares));
            Ok(())
        }
    }
}

impl<T: Trait<I>, I: Instance> Module<T, I> {
    /// Set the ShareProfile
    fn set_profile(
        who: &T::AccountId,
        id: T::ShareId,
        new: &ShareProfile<T::Share>,
    ) -> DispatchResult {
        Profile::<T, I>::insert(who, id, new);
        Ok(())
    }
}

impl<T: Trait<I>, I: Instance> IDIsAvailable<T::ShareId> for Module<T, I> {
    fn id_is_available(id: T::ShareId) -> bool {
        None == <TotalIssuance<T, I>>::get(id)
    }
}

impl<T: Trait<I>, I: Instance> GenerateUniqueID<T::ShareId> for Module<T, I> {
    fn generate_unique_id(proposed_id: T::ShareId) -> T::ShareId {
        let generated_id = if !Self::id_is_available(proposed_id) {
            let mut id_counter = <ShareIdCounter<T, I>>::get();
            while <TotalIssuance<T, I>>::get(id_counter).is_some() {
                // TODO: add overflow check here
                id_counter += 1.into();
            }
            <ShareIdCounter<T, I>>::put(id_counter + 1.into());
            id_counter
        } else {
            proposed_id
        };
        generated_id
    }
}

impl<T: Trait<I>, I: Instance> ShareRegistration<T::AccountId> for Module<T, I> {
    type GenesisAllocation = SimpleShareGenesis<T::AccountId, T::Share>;

    /// This registration logic is expensive and has complexity that is linear in the size of the group
    /// TODO: move to offchain worker scheduling whenever possible
    fn register(
        proposed_id: Self::ShareId,
        issuance: Self::GenesisAllocation,
    ) -> Result<Self::ShareId, DispatchError> {
        let share_id = Self::generate_unique_id(proposed_id);
        ensure!(
            issuance.clone().verify_shape(),
            Error::<T, I>::InitialIssuanceShapeIsInvalid
        );
        let mut shareholders: Vec<T::AccountId> = Vec::new();
        for account_share in issuance.account_ownership.iter() {
            let new_profile = ShareProfile {
                free: account_share.1,
                reserved: 0.into(),
            };
            shareholders.push(account_share.0.clone());
            Profile::<T, I>::insert(account_share.0.clone(), share_id, new_profile);
        }
        <TotalIssuance<T, I>>::insert(share_id, issuance.total);
        <ShareHolders<T, I>>::insert(share_id, shareholders);
        Ok(share_id)
    }
}

impl<T: Trait<I>, I: Instance> ReservableProfile<T::AccountId> for Module<T, I> {
    type Shares = T::Share;
    type ShareId = T::ShareId;

    fn reserve(
        who: &T::AccountId,
        share_id: Self::ShareId,
        amount: Option<Self::Shares>,
    ) -> Result<Self::Shares, DispatchError> {
        let fetched_profile = Profile::<T, I>::get(who, share_id);
        ensure!(
            fetched_profile.is_some(),
            Error::<T, I>::ProfileNotInstantiated
        );
        let old_profile = fetched_profile.expect("checked existence of inner value in line above");

        let amount = if let Some(amt) = amount {
            // specific amount requested for reservation
            ensure!(
                old_profile.free >= amt,
                Error::<T, I>::CannotAffordToReserveShareBond
            );
            amt
        } else {
            // reserve all shares that are free
            old_profile.free
        };

        // calculate new share profile fields
        let new_free = old_profile.free - amount;
        let new_reserved = old_profile.reserved + amount;
        // instantiate new share profile
        let new_share_profile = ShareProfile {
            free: new_free,
            reserved: new_reserved,
        };
        // set new share profile for `who`
        Self::set_profile(who, share_id, &new_share_profile)?;
        Ok(amount)
    }

    fn unreserve(
        who: &T::AccountId,
        share_id: Self::ShareId,
        amount: Option<Self::Shares>,
    ) -> Result<Self::Shares, DispatchError> {
        let fetched_profile = Profile::<T, I>::get(who, share_id);
        ensure!(
            fetched_profile.is_some(),
            Error::<T, I>::ProfileNotInstantiated
        );
        let old_profile = fetched_profile.expect("checked existence of inner value in line above");

        let amount = if let Some(amt) = amount {
            // specific amount requested for unreservation
            ensure!(
                old_profile.reserved >= amt,
                Error::<T, I>::CannotAffordToUnreserveShareBond
            );
            amt
        } else {
            // unreserve all shares that are reserved
            old_profile.reserved
        };

        // calculate new share profile fields
        let new_free = old_profile.free + amount;
        let new_reserved = old_profile.reserved - amount;
        // instantiate new share profile
        let new_share_profile = ShareProfile {
            free: new_free,
            reserved: new_reserved,
        };

        Self::set_profile(who, share_id, &new_share_profile)?;
        Ok(amount)
    }
}

impl<T: Trait<I>, I: Instance> ShareBank<T::AccountId> for Module<T, I> {
    fn outstanding_shares(id: Self::ShareId) -> T::Share {
        if let Some(amount) = <TotalIssuance<T, I>>::get(id) {
            amount
        } else {
            0.into()
        }
    }

    fn shareholder_membership(id: Self::ShareId) -> Result<Vec<T::AccountId>, DispatchError> {
        if let Some(membership) = <ShareHolders<T, I>>::get(id) {
            Ok(membership)
        } else {
            Err(Error::<T, I>::ShareHolderMembershipUninitialized.into())
        }
    }

    fn issue(owner: &T::AccountId, id: Self::ShareId, amount: Self::Shares) -> DispatchResult {
        let current_issuance =
            <TotalIssuance<T, I>>::get(id).ok_or(Error::<T, I>::ShareTypeNotRegistered)?;
        // update total issuance
        let new_amount = current_issuance + amount;
        <TotalIssuance<T, I>>::insert(id, new_amount);
        // update the recipient's share profile
        let old_share_profile = Profile::<T, I>::get(owner, id);
        let new_share_profile = if let Some(old_profile) = old_share_profile {
            // TODO: checked_add
            let new_free = old_profile.free + amount;
            ShareProfile {
                free: new_free,
                ..old_profile
            }
        } else {
            ShareProfile {
                free: amount,
                ..Default::default()
            }
        };
        Profile::<T, I>::insert(owner, id, new_share_profile);
        Ok(())
    }

    fn buyback(owner: &T::AccountId, id: Self::ShareId, amount: Self::Shares) -> DispatchResult {
        // (1) change total issuance
        let current_issuance =
            <TotalIssuance<T, I>>::get(id).ok_or(Error::<T, I>::ShareTypeNotRegistered)?;
        // (2) change owner's profile
        let profile =
            Profile::<T, I>::get(owner, id).ok_or(Error::<T, I>::ProfileNotInstantiated)?;
        // enforce invariant that the owner's shares must be reserved in order to burnt
        ensure!(
            profile.reserved >= amount,
            Error::<T, I>::CanOnlyBurnReservedShares
        );
        // ..(1)
        let new_amount = current_issuance - amount;
        <TotalIssuance<T, I>>::insert(id, new_amount);
        // ..(2)
        let new_reserved = profile.reserved - amount;
        let new_profile = ShareProfile {
            reserved: new_reserved,
            ..profile
        };
        Profile::<T, I>::insert(owner, id, new_profile);
        Ok(())
    }
}

// For testing purposes only
impl<T: Trait<I>, I: Instance> GetProfile<T::AccountId> for Module<T, I> {
    fn get_free_shares(
        who: &T::AccountId,
        share_id: Self::ShareId,
    ) -> Result<T::Share, DispatchError> {
        let wrapped_profile = Profile::<T, I>::get(who, share_id);
        if let Some(profile) = wrapped_profile {
            Ok(profile.free)
        } else {
            Err(Error::<T, I>::ProfileNotInstantiated.into())
        }
    }

    fn get_reserved_shares(
        who: &T::AccountId,
        share_id: Self::ShareId,
    ) -> Result<T::Share, DispatchError> {
        let wrapped_profile = Profile::<T, I>::get(who, share_id);
        if let Some(profile) = wrapped_profile {
            Ok(profile.reserved)
        } else {
            Err(Error::<T, I>::ProfileNotInstantiated.into())
        }
    }
}
