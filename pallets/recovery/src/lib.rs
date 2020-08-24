#![recursion_limit = "256"]
#![cfg_attr(not(feature = "std"), no_std)]
//! Password recovery pallet

use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        ReservableCurrency,
    },
    Parameter,
};
use frame_system::{
    ensure_signed,
    Trait as System,
};
use orml_utilities::OrderedSet;
use parity_scale_codec::Codec;
use sp_runtime::{
    traits::{
        AccountIdConversion,
        AtLeast32BitUnsigned,
        CheckedSub,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchError,
    DispatchResult,
    ModuleId,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::sss::{
    Commit,
    Relation,
    RelationState,
    SSSState,
    SecretState,
};

type SecretShare = Vec<u8>;
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as System>::AccountId>>::Balance;
type SecretSt<T> = SecretState<
    <T as Trait>::SecretId,
    <T as System>::AccountId,
    <T as Trait>::RoundId,
    BalanceOf<T>,
    SSSState,
>;
type History<T> = Relation<
    (<T as Trait>::SecretId, <T as System>::AccountId),
    Commit<<T as Trait>::RoundId, <T as System>::Hash, SecretShare>,
    RelationState,
>;
pub trait Trait: System {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// The secret identifier
    type SecretId: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// Round identifier
    type RoundId: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// The module account
    type Pool: Get<ModuleId>;

    /// Currency type
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as System>::AccountId,
        <T as System>::Hash,
        <T as Trait>::SecretId,
        <T as Trait>::RoundId,
    {
        /// Poster, Secret ID
        SecretGroupInitialized(AccountId, SecretId),
        /// Secret ID, Revoked Secret Keeper
        RevokeInvitation(SecretId, AccountId),
        /// Keeper Commitment to Hash of Secret Share
        CommitSecretHash(AccountId, SecretId, RoundId, Hash),
        /// Poster requests recovery of Secret
        RequestRecovery(AccountId, SecretId, RoundId),
        /// Keeper Reveals Preimage of Hash to Claim Funds
        RevealPreimage(AccountId, SecretId, RoundId, SecretShare),
        /// New Round Starts
        NewRound(SecretId, RoundId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        SecretDNE,
        UserCannotAffordRequest,
        NotAuthorizedForSecret,
        CommitAlreadyMadeForMostRecentRound,
        BranchShouldNeverReach,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Vote {
        /// The nonce for unique secret id generation
        SecretIdCounter get(fn secret_id_counter): T::SecretId;

        /// The state of a secret
        pub Secrets get(fn secrets): map
            hasher(blake2_128_concat) T::SecretId => Option<SecretSt<T>>;
        /// Hash commitment of secret share
        pub Commits get(fn commits): double_map
            hasher(blake2_128_concat) T::SecretId,
            hasher(blake2_128_concat) T::AccountId => Option<History<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn invite_group(
            origin,
            pool: BalanceOf<T>,
            reserve_req: BalanceOf<T>,
            accounts: Vec<T::AccountId>,
        ) -> DispatchResult {
            let user = ensure_signed(origin)?;
            ensure!(T::Currency::free_balance(&user) > pool, Error::<T>::UserCannotAffordRequest);
            let potential_secret_holders = OrderedSet::from(accounts);
            let id = Self::generate_secret_uid();
            T::Currency::transfer(
                &user,
                &Self::secret_account_id(id),
                pool,
                ExistenceRequirement::KeepAlive,
            )?;
            potential_secret_holders.0.into_iter().for_each(|a: T::AccountId| {
                let c_history = History::<T>::new((id, a.clone()), OrderedSet::new(), RelationState::Unreserved);
                <Commits<T>>::insert(id, a.clone(), c_history);
            });
            let secret_st = SecretSt::<T>::new(id, user.clone(), Zero::zero(), reserve_req, SSSState::Unused);
            <Secrets<T>>::insert(id, secret_st);
            Self::deposit_event(RawEvent::SecretGroupInitialized(user, id));
            Ok(())
        }
        #[weight = 0]
        pub fn revoke_invitation(
            origin,
            secret_id: T::SecretId,
            account: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let secret = <Secrets<T>>::get(secret_id).ok_or(Error::<T>::SecretDNE)?;
            ensure!(secret.user() == caller, Error::<T>::NotAuthorizedForSecret);
            <Commits<T>>::remove(secret_id, account.clone());
            Self::deposit_event(RawEvent::RevokeInvitation(secret_id, account));
            Ok(())
        }
        #[weight = 0]
        pub fn increment_round(
            origin,
            secret_id: T::SecretId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let secret = <Secrets<T>>::get(secret_id).ok_or(Error::<T>::SecretDNE)?;
            ensure!(secret.user() == caller, Error::<T>::NotAuthorizedForSecret);
            let new_secret = secret.inc_round();
            let round = new_secret.round();
            <Secrets<T>>::insert(secret_id, new_secret);
            Self::deposit_event(RawEvent::NewRound(secret_id, round));
            Ok(())
        }
        #[weight = 0]
        pub fn commit_secret_hash(
            origin,
            secret_id: T::SecretId,
            hash: T::Hash,
        ) -> DispatchResult {
            let participant = ensure_signed(origin)?;
            let secret = <Secrets<T>>::get(secret_id).ok_or(Error::<T>::SecretDNE)?;
            // TODO
            Self::deposit_event(RawEvent::CommitSecretHash(participant, secret_id, secret.round(), hash));
            Ok(())
        }
        #[weight = 0]
        pub fn reveal_secret_preimage(
            origin,
            secret_id: T::SecretId,
            preimage: SecretShare,
        ) -> DispatchResult {
            todo!()
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn secret_account_id(index: T::SecretId) -> T::AccountId {
        T::Pool::get().into_sub_account(index)
    }
    fn generate_secret_uid() -> T::SecretId {
        let mut secret_counter = <SecretIdCounter<T>>::get() + 1u32.into();
        while <Secrets<T>>::get(secret_counter).is_some() {
            secret_counter += 1u32.into();
        }
        <SecretIdCounter<T>>::put(secret_counter);
        secret_counter
    }
}
