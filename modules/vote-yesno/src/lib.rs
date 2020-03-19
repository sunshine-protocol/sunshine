#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get, Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member},
    DispatchError, DispatchResult, Permill,
};
use sp_std::{fmt::Debug, prelude::*};
use util::{
    proposal::ProposalType,
    traits::{
        ApplyVote, Approved, CalculateVoteThreshold, CheckVoteStatus, GenerateUniqueID,
        IDIsAvailable, MintableSignal, OpenVote, ReservableProfile, SetThresholdConfig, ShareBank,
        VoteOnProposal,
    },
    vote::{Outcome, ThresholdConfig, VoteState, VoteThreshold, VoterYesNoView, YesNoVote},
};

/// The shares type that is converted into signal for each instance of this module
pub type SharesOf<T> =
    <<T as Trait>::ShareData as ReservableProfile<<T as frame_system::Trait>::AccountId>>::Shares;

/// The share identifier type
pub type ShareId<T> =
    <<T as Trait>::ShareData as ReservableProfile<<T as frame_system::Trait>::AccountId>>::ShareId;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The native voting power type
    type Signal: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + From<SharesOf<Self>>
        + Into<SharesOf<Self>>;

    /// The identifier for each vote; ProposalId => Vec<VoteId> s.t. sum(VoteId::Outcomes) => ProposalId::Outcome
    type VoteId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    /// An instance of the shares module
    type ShareData: ReservableProfile<Self::AccountId> + ShareBank<Self::AccountId>;

    /// The default vote length
    type DefaultVoteLength: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait>::VoteId,
        ShareId = ShareId<T>,
    {
        NewVoteStarted(ShareId, VoteId),
        Voted(AccountId, VoteId),
        ThresholdRequirementSet(ShareId, ProposalType, Permill, Permill),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        VoteIdentityUnitialized,
        ShareMembershipUnitialized,
        CurrentBlockNumberNotMoreRecent,
        VoteStateUninitialized,
        /// This ensures that the outcome was initialized in order to allow `VoteOnProposal`
        VoteNotInitialized,
        /// This ensures that the user can only vote when the outcome is in voting
        CanOnlyVoteinVotingOutcome,
        VoterViewNotAccountedFor,
        /// Current time is past the time of the vote expiration so new votes are not accepted
        VotePastExpirationTimeSoVotesNotAccepted,
        NotEnoughSignalToVote,
        ThresholdConfigNotSetForProposalType,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as VoteYesNo {
        /// VoteId storage helper for unique id generation, see issue #62
        pub VoteIdCounter get(vote_id_counter): T::VoteId;

        /// Total signal available for each member for the vote in question
        /// - TODO: consider changing `VoteId` for `ProposalType` because I think that's how minting actually happens
        /// although in other modules, this level of specificity may be valuable if signal is configurable per vote_id
        /// ...right now it is only configurable in the context of (ShareId, ProposalType)
        pub MintedSignal get(minted_signal): double_map hasher(blake2_256) T::AccountId, hasher(blake2_256) T::VoteId  => Option<T::Signal>;

        /// The Collective ThresholdConfig, established through governance via _aggregation_ of individual preferences
        pub CollectiveThresholdConfig get(fn collective_vote_config): double_map
            hasher(blake2_256) ShareId<T>,
            hasher(blake2_256) ProposalType
            => Option<ThresholdConfig<Permill>>;

        /// The state of a vote (separate from outcome so that this can be purged if Outcome is not Voting)
        pub VoteStates get(fn vote_states): map hasher(blake2_256) T::VoteId => Option<VoteState<ShareId<T>, T::VoteId, T::Signal, T::BlockNumber>>;

        /// The outcome of a vote
        pub VoteOutcome get(fn vote_outcome): map hasher(blake2_256) T::VoteId => Option<Outcome>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        const DefaultVoteLength: T::BlockNumber = T::DefaultVoteLength::get();

        // I would like this method to be called in a different module, but from an instance of this module
        // - I think I'm going to have to add ProposalId as a type here and it should come from `bank` bc that's where proposals start
        // but does that introduce cyclic dependencies if each of these modules depends on each other
        fn create_vote(
            origin,
            proposal_type: ProposalType,
            vote_id: T::VoteId,
            share_id: ShareId<T>
        ) -> DispatchResult {
            // TODO: replace with origin check once I give a shit about permissions
            let _ = ensure_signed(origin)?;
            let new_vote_id = Self::open_vote(vote_id, share_id, proposal_type)?;
            Self::deposit_event(RawEvent::NewVoteStarted(share_id, new_vote_id));
            Ok(())
        }

        fn vote(
            origin,
            vote_id: T::VoteId,
            direction: VoterYesNoView,
            magnitude: Option<T::Signal>
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            Self::vote_on_proposal(voter.clone(), vote_id, direction, magnitude)?;
            Self::deposit_event(RawEvent::Voted(voter, vote_id));
            Ok(())
        }

        fn set_threshold_requirement(
            origin,
            share_id: ShareId<T>,
            proposal_type: ProposalType,
            passage_threshold_pct: Permill,
            turnout_threshold_pct: Permill,
        ) -> DispatchResult {
            // TODO: add permissioned check that aligns with share type admin
            let _ = ensure_signed(origin)?;
            Self::set_threshold_config(share_id, proposal_type, passage_threshold_pct, turnout_threshold_pct)?;
            Self::deposit_event(RawEvent::ThresholdRequirementSet(share_id, proposal_type, passage_threshold_pct, turnout_threshold_pct));
            Ok(())
        }
    }
}

impl<T: Trait> IDIsAvailable<T::VoteId> for Module<T> {
    fn id_is_available(id: T::VoteId) -> bool {
        None == <VoteStates<T>>::get(id)
    }
}

impl<T: Trait> GenerateUniqueID<T::VoteId> for Module<T> {
    fn generate_unique_id(proposed_id: T::VoteId) -> T::VoteId {
        if !Self::id_is_available(proposed_id) {
            let mut id_counter = <VoteIdCounter<T>>::get();
            while <VoteStates<T>>::get(id_counter).is_some() {
                // TODO: add overflow check here
                id_counter += 1.into();
            }
            <VoteIdCounter<T>>::put(id_counter + 1.into());
            id_counter
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> MintableSignal<T::AccountId, T::Signal> for Module<T> {
    /// Mints signal based on the conversion rate in storage (uses `SignalConversion` traits)
    /// - the amount parameter is an option such that `None` implies that as many shares as possible
    /// should be reserved in order to mint signal
    fn mint_signal(
        who: T::AccountId,
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
        amount: Option<T::Signal>,
    ) -> Result<T::Signal, DispatchError> {
        let shares_to_reserve: Option<SharesOf<T>> = if let Some(amt) = amount {
            Some(amt.into())
        } else {
            // reserve as many as possible
            None
        };
        let shares_reserved = T::ShareData::reserve(&who, share_id, shares_to_reserve)?;
        // could add more nuanced conversion logic here; see doc/sharetovote
        let minted_signal: T::Signal = shares_reserved.into();
        <MintedSignal<T>>::insert(who, vote_id, minted_signal);
        Ok(minted_signal)
    }

    /// This mints signal for all of the shareholders and reserves as many free shares as they have
    /// to do so. The cost scales with the size of the shareholder group (in number of AccountId members)
    /// because it mints for each share signal (by calling `mint_signal` with None for the amount parameter, to execute
    /// the default happy path of reserving as many shares as possible to mint the signal...)
    fn batch_mint_signal(
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
    ) -> Result<T::Signal, DispatchError> {
        let new_vote_group = T::ShareData::shareholder_membership(share_id)?;
        let mut total_minted_signal: T::Signal = 0.into();
        new_vote_group.iter().for_each(|who| {
            // does this propagate errors
            let minted_signal = Self::mint_signal(who.clone(), vote_id, share_id, None);
            if let Ok(add_to_sum) = minted_signal {
                total_minted_signal += add_to_sum
            }
            // TODO: PROPER ERROR HANDLING HERE
        });
        Ok(total_minted_signal)
    }
}

impl<T: Trait> SetThresholdConfig<Permill> for Module<T> {
    type ThresholdConfig = ThresholdConfig<Permill>;

    // calls to this method should be limited by governance, TODO: preference aggregation
    fn set_threshold_config(
        share_id: Self::ShareId,
        proposal_type: Self::ProposalType,
        passage_threshold_pct: Permill,
        turnout_threshold_pct: Permill,
    ) -> DispatchResult {
        let threshold_config = ThresholdConfig::new(passage_threshold_pct, turnout_threshold_pct);
        <CollectiveThresholdConfig<T>>::insert(share_id, proposal_type, threshold_config);
        Ok(())
    }
}

impl<T: Trait> CalculateVoteThreshold<T::Signal, Permill> for Module<T> {
    type VoteThreshold = VoteThreshold<T::Signal, T::BlockNumber>;

    fn calculate_vote_threshold(
        threshold_config: Self::ThresholdConfig,
        possible_turnout: T::Signal,
    ) -> Self::VoteThreshold {
        // TODO: should add trait bound to ensure that these fields can multiply the possible turnout like this
        let support_required = threshold_config.passage_threshold_pct * possible_turnout;
        let turnout_required = threshold_config.turnout_threshold_pct * possible_turnout;
        let now = system::Module::<T>::block_number();
        Self::VoteThreshold::new(support_required, turnout_required, now)
    }
}

impl<T: Trait> OpenVote for Module<T> {
    type VoteId = T::VoteId;
    type ShareId = ShareId<T>;
    type ProposalType = ProposalType;

    fn open_vote(
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
        proposal_type: Self::ProposalType,
    ) -> Result<T::VoteId, DispatchError> {
        // need a standardized way to verify that this VoteId doesn't exist (across all maps)
        let new_vote_id = Self::generate_unique_id(vote_id);

        // calculate `initialized` and `expires` fields for vote state
        let now = system::Module::<T>::block_number();
        let ends = now + T::DefaultVoteLength::get();
        // mint signal for all of shareholders and get total possible turnout
        let total_possible_turnout = Self::batch_mint_signal(vote_id, share_id)?;
        // get the threshold configuration set in storage
        let threshold_config = <CollectiveThresholdConfig<T>>::get(share_id, proposal_type)
            .ok_or(Error::<T>::ThresholdConfigNotSetForProposalType)?;
        let threshold = Self::calculate_vote_threshold(threshold_config, total_possible_turnout);
        // TODO: replace this with a new method
        let new_vote_state = VoteState {
            electorate: share_id,
            vote_id: new_vote_id,
            // in_favor, against, and turnout are 0 by default
            proposal_type,
            threshold,
            initialized: now,
            expires: ends,
            ..Default::default()
        };

        // insert the VoteState
        <VoteStates<T>>::insert(new_vote_id, new_vote_state);
        // insert the current VoteOutcome (voting)
        <VoteOutcome<T>>::insert(new_vote_id, Outcome::default());

        Ok(new_vote_id)
    }

    // fn open_custom_vote() -- more configurable length than the above method
}

impl<T: Trait> ApplyVote for Module<T> {
    type Vote = YesNoVote<T::Signal>;
    type State = VoteState<ShareId<T>, T::VoteId, T::Signal, T::BlockNumber>;

    fn apply_vote(state: Self::State, vote: Self::Vote) -> Result<Self::State, DispatchError> {
        // update VoterStatus (which should wrap the vote for certain options)
        let new_state = match vote.direction {
            VoterYesNoView::InFavor => {
                // TODO: checked adds
                let new_in_favor = state.in_favor + vote.magnitude;
                let new_turnout = state.turnout + vote.magnitude;
                Self::State {
                    in_favor: new_in_favor,
                    turnout: new_turnout,
                    ..state
                }
            }
            VoterYesNoView::Against => {
                // TODO: checked adds
                let new_against = state.against + vote.magnitude;
                let new_turnout = state.turnout + vote.magnitude;
                Self::State {
                    against: new_against,
                    turnout: new_turnout,
                    ..state
                }
            }
            VoterYesNoView::Abstained => {
                // TODO: checked adds
                let new_turnout = state.turnout + vote.magnitude;
                Self::State {
                    turnout: new_turnout,
                    ..state
                }
            }
            _ => return Err(Error::<T>::VoterViewNotAccountedFor.into()),
        };
        Ok(new_state)
    }
}

// TODO => if approved, close the vote (and this logic should be associated with outcome)
impl<T: Trait> CheckVoteStatus for Module<T> {
    type Outcome = Outcome;

    // TO SEE IF THE OUTCOME HAS CHANGED
    fn check_vote_outcome(state: Self::State) -> Result<Self::Outcome, DispatchError> {
        // trait bound on Self::State ensures that it implements Approved so that's all we have for now
        if state.approved() {
            // this should trigger a state change in a different module (ie bank)
            // see https://substrate.dev/docs/en/tutorials/adding-a-module-to-your-runtime#adding-runtime-hooks
            return Ok(Outcome::Approved);
        }
        // TODO: add rejection outcome when we add the Rejected trait implementation for VoteState and bound to Self::State
        Ok(Outcome::Voting)
    }

    // TO SEE IF THE VOTE HAS EXPIRED
    fn check_vote_expired(state: Self::State) -> bool {
        let now = system::Module::<T>::block_number();
        state.expires < now
    }
}

impl<T: Trait> VoteOnProposal<T::AccountId> for Module<T> {
    type Direction = VoterYesNoView;
    type Magnitude = T::Signal;

    fn vote_on_proposal(
        voter: T::AccountId,
        vote_id: Self::VoteId,
        direction: Self::Direction,
        magnitude: Option<Self::Magnitude>,
    ) -> DispatchResult {
        // check that voting is permitted based on current outcome
        let current_outcome =
            <VoteOutcome<T>>::get(vote_id).ok_or(Error::<T>::VoteNotInitialized)?;
        ensure!(
            current_outcome == Outcome::Voting,
            Error::<T>::CanOnlyVoteinVotingOutcome
        );
        // get the vote state
        let current_vote_state =
            <VoteStates<T>>::get(vote_id).ok_or(Error::<T>::VoteStateUninitialized)?;
        // check that the vote has not expired (could be commented out to not enforce if decided to not enforce)
        ensure!(
            !Self::check_vote_expired(current_vote_state.clone()),
            Error::<T>::VotePastExpirationTimeSoVotesNotAccepted
        );
        let mintable_signal =
            <MintedSignal<T>>::get(voter, vote_id).ok_or(Error::<T>::NotEnoughSignalToVote)?;
        let minted_signal = if let Some(mag) = magnitude {
            ensure!(mintable_signal >= mag, Error::<T>::NotEnoughSignalToVote);
            mag
        } else {
            mintable_signal
        };
        // form the vote
        let vote = Self::Vote {
            direction,
            magnitude: minted_signal,
        };
        // get the new state by applying the vote
        let new_state = Self::apply_vote(current_vote_state, vote)?;
        // place new vote state in storage
        <VoteStates<T>>::insert(vote_id, new_state.clone());
        // get the new outcome using the vote_status
        let new_outcome = Self::check_vote_outcome(new_state)?;
        // insert new outcome
        <VoteOutcome<T>>::insert(vote_id, new_outcome);
        Ok(())
    }
}
