#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, CheckedSub, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult, Permill,
};
use sp_std::{fmt::Debug, prelude::*};
use util::{
    traits::{
        AccessGenesis, Apply, ApplyVote, Approved, ChainSudoPermissions, ChangeGroupMembership,
        CheckVoteStatus, GenerateUniqueID, GetFlatShareGroup, GetGroupSize, GetMagnitude,
        GetVoteOutcome, GroupMembership, IDIsAvailable, LockableProfile, MintableSignal,
        OpenShareGroupVote, OrganizationSupervisorPermissions, ReservableProfile, Revert,
        ShareBank, SubGroupSupervisorPermissions, ThresholdVote, VoteOnProposal, VoteVector,
        WeightedShareGroup,
    },
    uuid::UUID2,
    voteyesno::{
        SupportedVoteTypes, ThresholdConfig, VoteOutcome, VoteState, VoterYesNoView, YesNoVote,
    },
};

/// The organization identifier type
pub type OrgId = u32;
/// The share identifier type
pub type ShareId = u32;
/// Reference to generic data stored on IPFS
pub type IpfsReference = Vec<u8>;

/// The shares type used directly for vote strength in this module
pub type SharesOf<T> = <<T as Trait>::WeightedShareData as WeightedShareGroup<
    <T as frame_system::Trait>::AccountId,
>>::Shares;

/// The vote identifier type
pub type VoteId = u32;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The native unit for voting power in this module
    type Signal: Parameter
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
        + From<SharesOf<Self>>;

    /// An instance of the organizational membership module
    type OrgData: GetGroupSize<GroupId = u32>
        + GroupMembership<Self::AccountId>
        + IDIsAvailable<OrgId>
        + GenerateUniqueID<OrgId>
        + ChainSudoPermissions<Self::AccountId>
        + OrganizationSupervisorPermissions<u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>;

    /// An instance of `SharesMembership` for flat membership groups
    type FlatShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId, GroupId = UUID2>
        + SubGroupSupervisorPermissions<u32, u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>
        + GetFlatShareGroup<Self::AccountId>;

    /// An instance of `SharesAtomic` for weighted membership groups
    type WeightedShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId>
        + WeightedShareGroup<Self::AccountId>
        + ShareBank<Self::AccountId>
        + ReservableProfile<Self::AccountId>
        + LockableProfile<Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        NewVoteStarted(OrgId, ShareId, VoteId),
        Voted(VoteId, AccountId),
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
        /// Current time is past the time of the vote expiration so new votes are not accepted
        VotePastExpirationTimeSoVotesNotAccepted,
        NotEnoughSignalToVote,
        /// Tried to get voting outcome but returned None from storage
        NoOutcomeAssociatedWithVoteID,
        NewVoteCannotReplaceOldVote,
        /// Local Auths
        NotAuthorizedToCreateVoteForOrganization,
        NotAuthorizedToSubmitVoteForUser,
        FlatShareGroupDNEsoCantBatchSignal,
        WeightedShareGroupDNEsoCantBatchSignal,
        EnsureThatTotalSignalIssuanceEqualsSum,
        // the logic for this match arms hasnt been written for
        // the SupportedVoteType enum variants
        VoteTypeNotYetSupported,
        NoVoteStateForOutcomeQuery,
        NoVoteStateForVoteRequest,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as VoteYesNo {
        pub VoteIdCounter get(fn vote_id_counter): VoteId;

        /// The number of open votes
        /// TODO: sync this with create_vote and close_vote
        /// TODO2: add same thing to vote-petition
        pub OpenVoteCounter get(fn open_vote_counter): u32;

        /// The state of a vote (separate from outcome so that this can be purged if Outcome is not Voting)
        pub VoteStates get(fn vote_states): map
            hasher(opaque_blake2_256) VoteId => Option<VoteState<T::Signal, Permill, T::BlockNumber>>;

        pub TotalSignalIssuance get(fn total_signal_issuance): map
            hasher(opaque_blake2_256) VoteId => Option<T::Signal>;

        /// Total signal available for each member for the vote in question
        pub MintedSignal get(fn minted_signal): double_map
            hasher(opaque_blake2_256) VoteId,
            hasher(opaque_blake2_256) T::AccountId  => Option<T::Signal>;

        /// Tracks all votes
        pub VoteLogger get(fn vote_logger): double_map
            hasher(opaque_blake2_256) VoteId,
            hasher(opaque_blake2_256) T::AccountId  => Option<YesNoVote<T::Signal, IpfsReference>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn create_share_weighted_percentage_threshold_vote(
            origin,
            organization: OrgId,
            share_id: ShareId,
            passage_threshold_pct: Permill,
            turnout_threshold_pct: Permill,
        ) -> DispatchResult {
            let vote_creator = ensure_signed(origin)?;
            // default authentication is organization supervisor or sudo key
            let authentication: bool = <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
                u32,
                <T as frame_system::Trait>::AccountId,
            >>::is_organization_supervisor(organization, &vote_creator) ||
            <<T as Trait>::OrgData as ChainSudoPermissions<
                <T as frame_system::Trait>::AccountId,
            >>::is_sudo_key(&vote_creator);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreateVoteForOrganization);
            let threshold_config = ThresholdConfig::new_percentage_threshold(passage_threshold_pct, turnout_threshold_pct);
            // default share weighted
            let new_vote_id = Self::open_share_group_vote(organization, share_id, SupportedVoteTypes::ShareWeighted, threshold_config, None)?;
            // emit event
            Self::deposit_event(RawEvent::NewVoteStarted(organization, share_id, new_vote_id));
            Ok(())
        }

        #[weight = 0]
        pub fn create_share_weighted_count_threshold_vote(
            origin,
            organization: OrgId,
            share_id: ShareId,
            support_requirement: T::Signal,
            turnout_requirement: T::Signal,
        ) -> DispatchResult {
            let vote_creator = ensure_signed(origin)?;
            // default authentication is organization supervisor or sudo key
            let authentication: bool = <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
                u32,
                <T as frame_system::Trait>::AccountId,
            >>::is_organization_supervisor(organization, &vote_creator) ||
            <<T as Trait>::OrgData as ChainSudoPermissions<
                <T as frame_system::Trait>::AccountId,
            >>::is_sudo_key(&vote_creator);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreateVoteForOrganization);
            let threshold_config = ThresholdConfig::new_signal_count_threshold(support_requirement, turnout_requirement);
            // share weighted count threshold
            let new_vote_id = Self::open_share_group_vote(organization, share_id, SupportedVoteTypes::ShareWeighted, threshold_config, None)?;
            // emit event
            Self::deposit_event(RawEvent::NewVoteStarted(organization, share_id, new_vote_id));
            Ok(())
        }

        #[weight = 0]
        pub fn create_1p1v_percentage_threshold_vote(
            origin,
            organization: OrgId,
            share_id: ShareId,
            support_requirement: Permill,
            turnout_requirement: Permill,
        ) -> DispatchResult {
            let vote_creator = ensure_signed(origin)?;
            // default authentication is organization supervisor or sudo key
            let authentication: bool = <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
                u32,
                <T as frame_system::Trait>::AccountId,
            >>::is_organization_supervisor(organization, &vote_creator) ||
            <<T as Trait>::OrgData as ChainSudoPermissions<
                <T as frame_system::Trait>::AccountId,
            >>::is_sudo_key(&vote_creator);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreateVoteForOrganization);
            let threshold_config = ThresholdConfig::new_percentage_threshold(support_requirement, turnout_requirement);
            // share weighted count threshold
            let new_vote_id = Self::open_share_group_vote(organization, share_id, SupportedVoteTypes::OneAccountOneVote, threshold_config, None)?;
            // emit event
            Self::deposit_event(RawEvent::NewVoteStarted(organization, share_id, new_vote_id));
            Ok(())
        }

        #[weight = 0]
        pub fn create_1p1v_count_threshold_vote(
            origin,
            organization: OrgId,
            share_id: ShareId,
            support_requirement: T::Signal,
            turnout_requirement: T::Signal,
        ) -> DispatchResult {
            let vote_creator = ensure_signed(origin)?;
            // default authentication is organization supervisor or sudo key
            let authentication: bool = <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
                u32,
                <T as frame_system::Trait>::AccountId,
            >>::is_organization_supervisor(organization, &vote_creator) ||
            <<T as Trait>::OrgData as ChainSudoPermissions<
                <T as frame_system::Trait>::AccountId,
            >>::is_sudo_key(&vote_creator);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreateVoteForOrganization);
            let threshold_config = ThresholdConfig::new_signal_count_threshold(support_requirement, turnout_requirement);
            // one account one vote as per the API
            let new_vote_id = Self::open_share_group_vote(organization, share_id, SupportedVoteTypes::OneAccountOneVote, threshold_config, None)?;
            // emit event
            Self::deposit_event(RawEvent::NewVoteStarted(organization, share_id, new_vote_id));
            Ok(())
        }

        #[weight = 0]
        pub fn submit_vote(
            origin,
            vote_id: VoteId,
            direction: VoterYesNoView,
            magnitude: Option<T::Signal>,
            justification: Option<IpfsReference>,
        ) -> DispatchResult {
            let voter = ensure_signed(origin)?;
            Self::vote_on_proposal(vote_id, voter.clone(), direction, magnitude, justification)?;
            Self::deposit_event(RawEvent::Voted(vote_id, voter));
            Ok(())
        }
    }
}

impl<T: Trait> IDIsAvailable<VoteId> for Module<T> {
    fn id_is_available(id: VoteId) -> bool {
        <VoteStates<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<VoteId> for Module<T> {
    fn generate_unique_id() -> VoteId {
        let mut id_counter = VoteIdCounter::get() + 1u32;
        while <VoteStates<T>>::get(id_counter).is_some() {
            id_counter += 1u32;
        }
        VoteIdCounter::put(id_counter);
        id_counter
    }
}

impl<T: Trait> MintableSignal<T::AccountId, T::BlockNumber, Permill> for Module<T> {
    /// Mints a custom amount of signal
    /// - may be useful for resetting voting rights
    /// - should be heavily guarded and no public facing
    fn mint_custom_signal_for_account(vote_id: VoteId, who: &T::AccountId, signal: T::Signal) {
        <MintedSignal<T>>::insert(vote_id, who, signal);
    }

    fn batch_mint_signal_for_1p1v_share_group(
        organization: OrgId,
        share_id: ShareId,
    ) -> Result<(VoteId, T::Signal), DispatchError> {
        let new_vote_group = T::FlatShareData::get_organization_share_group(organization, share_id)
            .ok_or(Error::<T>::FlatShareGroupDNEsoCantBatchSignal)?;
        let total_minted: T::Signal = (new_vote_group.len() as u32).into();
        let vote_id = Self::generate_unique_id();
        <TotalSignalIssuance<T>>::insert(vote_id, total_minted);
        let one_signal: T::Signal = 1u32.into();
        let _ = new_vote_group.into_iter().for_each(|who| {
            // mint signal for individual
            <MintedSignal<T>>::insert(vote_id, who, one_signal);
        });
        Ok((vote_id, total_minted))
    }

    /// This mints signal for all of the shareholders based on the vote type.
    /// The cost scales with the size of the shareholder group (in number of AccountId members)
    /// because it mints signal for each account in the (org, share_id) share group
    fn batch_mint_signal_for_weighted_share_group(
        organization: OrgId,
        share_id: ShareId,
    ) -> Result<(VoteId, T::Signal), DispatchError> {
        let new_vote_group = T::WeightedShareData::shareholder_membership(organization, share_id)
            .ok_or(Error::<T>::WeightedShareGroupDNEsoCantBatchSignal)?;
        // insert total issuance
        let mut total_minted = T::Signal::zero();
        let vote_id = Self::generate_unique_id();
        let _ = new_vote_group
            .account_ownership()
            .iter() // we don't need amt because we assume full reservation by default
            .map(|(who, _)| -> Result<(), DispatchError> {
                let reservation_context =
                    T::WeightedShareData::reserve(organization, share_id, &who, None)?;
                let minted_signal: T::Signal = reservation_context.get_magnitude().into();
                total_minted += minted_signal;
                // mint signal for individual
                <MintedSignal<T>>::insert(vote_id, who, minted_signal);
                Ok(())
            })
            .collect::<Result<(), DispatchError>>()?;
        <TotalSignalIssuance<T>>::insert(vote_id, total_minted);
        Ok((vote_id, total_minted))
    }
}

impl<T: Trait> GetVoteOutcome for Module<T> {
    type VoteId = VoteId;
    type Outcome = VoteOutcome;
    fn get_vote_outcome(vote_id: VoteId) -> Result<Self::Outcome, DispatchError> {
        let vote_state =
            <VoteStates<T>>::get(vote_id).ok_or(Error::<T>::NoVoteStateForOutcomeQuery)?;
        Ok(vote_state.outcome())
    }
}

impl<T: Trait> ThresholdVote for Module<T> {
    type Signal = T::Signal;
}

impl<T: Trait> OpenShareGroupVote<T::AccountId, T::BlockNumber, Permill> for Module<T> {
    type ThresholdConfig = ThresholdConfig<T::Signal, Permill>;
    type VoteType = SupportedVoteTypes<T::Signal>;
    fn open_share_group_vote(
        organization: OrgId,
        share_id: ShareId,
        vote_type: Self::VoteType,
        threshold_config: Self::ThresholdConfig,
        duration: Option<T::BlockNumber>,
    ) -> Result<VoteId, DispatchError> {
        // calculate `initialized` and `expires` fields for vote state
        let now = system::Module::<T>::block_number();
        let ends: Option<T::BlockNumber> = if let Some(time_to_add) = duration {
            Some(now + time_to_add)
        } else {
            None
        };
        // match here on the vote type for deciding which batch signal mint issuance strategy to follow
        // mint signal for all of shareholders based on their share weight and the vote type
        let (new_vote_id, total_possible_turnout) = match vote_type {
            SupportedVoteTypes::OneAccountOneVote => {
                Self::batch_mint_signal_for_1p1v_share_group(organization, share_id)?
            }
            SupportedVoteTypes::ShareWeighted => {
                Self::batch_mint_signal_for_weighted_share_group(organization, share_id)?
            }
            _ => return Err(Error::<T>::VoteTypeNotYetSupported.into()),
        };
        // instantiate new VoteState with threshold and temporal metadata
        let new_vote_state = VoteState::new(total_possible_turnout, threshold_config, now, ends);
        // insert the VoteState
        <VoteStates<T>>::insert(new_vote_id, new_vote_state);
        Ok(new_vote_id)
    }
}

impl<T: Trait> ApplyVote for Module<T> {
    type Direction = VoterYesNoView;
    type Vote = YesNoVote<T::Signal, IpfsReference>;
    type State = VoteState<T::Signal, Permill, T::BlockNumber>;

    fn apply_vote(
        state: Self::State,
        new_vote: Self::Vote,
        old_vote: Option<Self::Vote>,
    ) -> Result<(Self::State, Option<(bool, Self::Signal)>), DispatchError> {
        // TODO: return the difference in magnitude between these two +-
        // -> the whole point is to prevent re-voting
        let mut surplus_signal: Option<(bool, Self::Signal)> = None;
        let new_state = if let Some(unwrapped_old_vote) = old_vote {
            if unwrapped_old_vote.magnitude() >= unwrapped_old_vote.magnitude() {
                let difference = unwrapped_old_vote.magnitude() - unwrapped_old_vote.magnitude();
                surplus_signal = Some((true, difference));
            } else {
                let difference = unwrapped_old_vote.magnitude() - unwrapped_old_vote.magnitude();
                surplus_signal = Some((false, difference));
            }
            state.revert(unwrapped_old_vote)
        } else {
            state
        };
        Ok((new_state.apply(new_vote), surplus_signal))
    }
}

// TODO => if approved, close the vote (and this logic should be associated with outcome)
impl<T: Trait> CheckVoteStatus for Module<T> {
    // TO SEE IF THE OUTCOME HAS CHANGED
    fn check_vote_outcome(state: Self::State) -> Result<Self::Outcome, DispatchError> {
        let current_block: T::BlockNumber = <frame_system::Module<T>>::block_number();
        let past_ends: bool = if let Some(end_time) = state.expires() {
            end_time <= current_block
        } else {
            true
        };
        // define new outcome based on any changes to the petition state
        let new_outcome = if state.approved() && !past_ends {
            VoteOutcome::Approved
        } else {
            // default
            VoteOutcome::Voting
        };
        Ok(new_outcome)
    }

    // TO SEE IF THE VOTE HAS EXPIRED
    fn check_vote_expired(state: Self::State) -> bool {
        let now = system::Module::<T>::block_number();
        if let Some(n) = state.expires() {
            return n < now;
        }
        false
    }
}

impl<T: Trait> VoteOnProposal<T::AccountId, IpfsReference, T::BlockNumber, Permill> for Module<T> {
    fn vote_on_proposal(
        vote_id: VoteId,
        voter: T::AccountId,
        direction: Self::Direction,
        magnitude: Option<T::Signal>,
        justification: Option<IpfsReference>,
    ) -> DispatchResult {
        // get the vote state
        let vote_state =
            <VoteStates<T>>::get(vote_id).ok_or(Error::<T>::NoVoteStateForVoteRequest)?;
        // check that the vote has not expired (could be commented out to not enforce if decided to not enforce)
        ensure!(
            !Self::check_vote_expired(vote_state.clone()),
            Error::<T>::VotePastExpirationTimeSoVotesNotAccepted
        );
        // get the organization associated with this vote_state
        let mut mintable_signal = <MintedSignal<T>>::get(vote_id, voter.clone())
            .ok_or(Error::<T>::NotEnoughSignalToVote)?;
        let minted_signal = if let Some(mag) = magnitude {
            mag
        } else {
            mintable_signal
        };
        // form the new vote
        let new_vote = YesNoVote::new(minted_signal, direction, justification);
        // check if there is an existing vote and if so whether it should be swapped
        let old_vote = <VoteLogger<T>>::get(vote_id, voter.clone());
        // get the new state by applying the vote
        let (new_state, surplus_diff) = Self::apply_vote(vote_state, new_vote.clone(), old_vote)?;
        if let Some((extra_signal, amt)) = surplus_diff {
            if extra_signal {
                mintable_signal += amt;
            } else {
                ensure!(mintable_signal >= amt, Error::<T>::NotEnoughSignalToVote);
                mintable_signal -= amt;
            }
        } else {
            if let Some(mag) = magnitude {
                ensure!(mintable_signal >= mag, Error::<T>::NotEnoughSignalToVote);
                mintable_signal -= mag
            } else {
                // full minted signal used in this vote
                mintable_signal = 0u32.into();
            }
        }
        // set the new vote for the voter's profile
        <VoteLogger<T>>::insert(vote_id, voter.clone(), new_vote);
        // commit new vote state to storage
        <VoteStates<T>>::insert(vote_id, new_state.clone());
        <MintedSignal<T>>::insert(vote_id, voter, mintable_signal);
        Ok(())
    }
}
