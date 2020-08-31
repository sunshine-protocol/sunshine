#![recursion_limit = "256"]
#![cfg_attr(not(feature = "std"), no_std)]
//! Rank Vote Module

use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
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
        AtLeast32BitUnsigned,
        CheckedSub,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchResult,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::rank::{
    Ballot,
    BallotState,
    VoteBoard,
    VoteState,
};

pub type Vote<T> = Ballot<
    (<T as Trait>::VoteId, <T as System>::AccountId),
    <T as System>::AccountId,
    <T as Trait>::Signal,
    BallotState,
>;
pub type VoteInfo<T> = VoteBoard<
    <T as Trait>::VoteId,
    <T as Trait>::Cid,
    <T as System>::AccountId,
    <T as Trait>::Signal,
    VoteState,
>;
pub trait Trait: System {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid reference type
    type Cid: Parameter + Member + Default;

    /// The vote identifier
    type VoteId: Parameter
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

    /// The metric for voting power
    type Signal: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + CheckedSub
        + Zero;
}

decl_event!(
    pub enum Event<T>
    where
        <T as System>::AccountId,
        <T as Trait>::VoteId,
    {
        VoteOpened(AccountId, VoteId),
        VoteLocked(VoteId),
        VoteClosed(AccountId, VoteId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        VoteDNE,
        NotAuthorizedToClose,
        NotAuthorizedToLock,
        NotAuthorizedToVote,
        MustBeOpenToLock,
        LockedVotesDoNotAcceptVotes,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Rank {
        /// The nonce for unique vote id generation
        VoteIdCounter get(fn vote_id_counter): T::VoteId;

        /// The state of a vote, including its rules
        pub VoteStates get(fn vote_states): map
            hasher(blake2_128_concat) T::VoteId => Option<VoteInfo<T>>;

        /// Total signal minted for the vote; sum of all participant signal for the vote
        pub TotalSignal get(fn total_signal): map
            hasher(blake2_128_concat) T::VoteId => Option<T::Signal>;

        /// Tracks all votes and signal for each participating account
        pub VoteLogger get(fn vote_logger): double_map
            hasher(blake2_128_concat) T::VoteId,
            hasher(blake2_128_concat) T::AccountId  => Option<Vote<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn open(
            origin,
            topic: T::Cid,
            candidates: Vec<T::AccountId>,
            voters: Vec<(T::AccountId, T::Signal)>,
        ) -> DispatchResult {
            let vote_creator = ensure_signed(origin)?;
            let id = Self::mint_signal(OrderedSet::from(voters));
            let zero_candidates = candidates.into_iter().map(|c| (c, Zero::zero())).collect::<Vec<(T::AccountId, T::Signal)>>();
            let vote_info = VoteInfo::<T>::new(id, topic, vote_creator.clone(), OrderedSet::from(zero_candidates));
            <VoteStates<T>>::insert(id, vote_info);
            Self::deposit_event(RawEvent::VoteOpened(vote_creator, id));
            Ok(())
        }
        #[weight = 0]
        pub fn close(
            origin,
            id: T::VoteId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let vote_state = <VoteStates<T>>::get(id).ok_or(Error::<T>::VoteDNE)?;
            ensure!(
                vote_state.is_controller(&caller),
                Error::<T>::NotAuthorizedToClose
            );
            <VoteStates<T>>::remove(id);
            <TotalSignal<T>>::remove(id);
            <VoteLogger<T>>::remove_prefix(id);
            Self::deposit_event(RawEvent::VoteClosed(caller, id));
            Ok(())
        }
        #[weight = 0]
        pub fn lock(
            origin,
            id: T::VoteId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let vote_state = <VoteStates<T>>::get(id).ok_or(Error::<T>::VoteDNE)?;
            ensure!(
                vote_state.state() == VoteState::Open,
                Error::<T>::MustBeOpenToLock
            );
            ensure!(
                vote_state.is_controller(&caller),
                Error::<T>::NotAuthorizedToLock
            );
            <VoteStates<T>>::insert(id, vote_state.lock());
            Self::deposit_event(RawEvent::VoteLocked(id));
            Ok(())
        }
        #[weight = 0]
        pub fn vote(
            origin,
            vote_id: T::VoteId,
            votes: Vec<(T::AccountId, T::Signal)>,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            let vote_state = <VoteStates<T>>::get(vote_id).ok_or(Error::<T>::VoteDNE)?;
            ensure!(vote_state.state() == VoteState::Open, Error::<T>::LockedVotesDoNotAcceptVotes);
            let _ = <VoteLogger<T>>::get(vote_id, &voter).ok_or(Error::<T>::NotAuthorizedToVote)?;
            // TODO: update vote state if the vote is applied to votelogger correctly
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn generate_vote_uid() -> T::VoteId {
        let mut vote_counter = <VoteIdCounter<T>>::get() + 1u32.into();
        while <VoteStates<T>>::get(vote_counter).is_some() {
            vote_counter += 1u32.into();
        }
        <VoteIdCounter<T>>::put(vote_counter);
        vote_counter
    }
}

impl<T: Trait> Module<T> {
    pub fn mint_signal(
        voters: OrderedSet<(T::AccountId, T::Signal)>,
    ) -> T::VoteId {
        let id = Self::generate_vote_uid();
        let mut total_signal: T::Signal = Zero::zero();
        voters.0.into_iter().for_each(|(a, s)| {
            <VoteLogger<T>>::insert(id, a.clone(), Vote::<T>::new((id, a), s));
            total_signal += s;
        });
        <TotalSignal<T>>::insert(id, total_signal);
        id
    }
}
