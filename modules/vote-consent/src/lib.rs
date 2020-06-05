#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Simple module for collecting signatures from organizational share groups
//! - this is a simple vote machine, similar to `vote-yesno` but without any
//! share-weighted threshold or counting complexity

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, Parameter}; // storage::IterableStorageDoubleMap
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};
use util::{
    petition::{PetitionOutcome, PetitionSignature, PetitionState, PetitionView},
    traits::{
        Apply, Approved, ChainSudoPermissions, GenerateUniqueID, GetGroupSize, GetVoteOutcome,
        GroupMembership, IDIsAvailable, OpenPetition, OrganizationSupervisorPermissions, Rejected,
        SignPetition, UpdatePetition, UpdatePetitionTerms,
    },
};

/// The petition identifier
pub type PetitionId = u32;

pub trait Trait: frame_system::Trait + org::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The identifier for each vote for this module
    type PetitionId: Parameter
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
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
        <T as Trait>::PetitionId,
    {
        NewPetitionStarted(AccountId, OrgId, PetitionId),
        UserSignedPetition(PetitionId, AccountId, PetitionView<IpfsReference>, PetitionOutcome),
        PetitionUpdated(AccountId, PetitionId, IpfsReference),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NotAuthorizedToCreatePetition,
        NotAuthorizedToUpdatePetition,
        PetitionDoesNotSatisfyCreationConstraints,
        CantSignBecauseAccountNotFoundInShareGroup,
        CannotSignIfPetitionStateDNE,
        CannotUpdateIfPetitionStateDNE,
        CannotGetStatusIfPetitionStateDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as VoteConsent {
        /// The nonce for local uid generation
        PetitionIdNonce get(fn petition_id_nonce): T::PetitionId;

        /// The number of ongoing petitions
        pub PetitionCount get(fn petition_count): u32;

        /// The state of ongoing petitions
        pub PetitionStates get(fn petition_states): map
            hasher(opaque_blake2_256) T::PetitionId => Option<PetitionState<T::OrgId, T::IpfsReference, T::BlockNumber>>;

        /// The signatures of participants in the petition
        pub SignatureLogger get(fn signature_logger): double_map
            hasher(opaque_blake2_256) T::PetitionId,
            hasher(opaque_blake2_256) T::AccountId => Option<PetitionSignature<T::AccountId, T::IpfsReference>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn create_petition(
            origin,
            organization: T::OrgId,
            topic: Option<T::IpfsReference>,
            required_support: u32,
            vetos_to_reject: u32,
            ends: Option<T::BlockNumber>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = <org::Module<T>>::is_sudo_key(&caller)
                || <org::Module<T>>::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreatePetition);
            // create and open the petition
            let petition_id = Self::open_petition(
                organization,
                topic,
                required_support,
                vetos_to_reject,
                ends,
            )?;
            Self::deposit_event(RawEvent::NewPetitionStarted(caller, organization, petition_id));
            Ok(())
        }
        #[weight = 0]
        fn direct_sign_petition(
            origin,
            petition_id: T::PetitionId,
            view: PetitionView<T::IpfsReference>,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            let outcome = Self::sign_petition(
                petition_id,
                signer.clone(),
                view.clone(),
            )?;
            Self::deposit_event(RawEvent::UserSignedPetition(petition_id, signer, view, outcome));
            Ok(())
        }
        #[weight = 0]
        fn update_petition_and_clear_state(
            origin,
            organization: T::OrgId,
            petition_id: T::PetitionId,
            new_topic: T::IpfsReference,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            Self::update_petition(petition_id, new_topic, true)?;
            // Self::deposit_event(RawEvent::PetitionUpdated(caller, organization, share_id, petition_id));
            Ok(())
        }
    }
}

impl<T: Trait> IDIsAvailable<T::PetitionId> for Module<T> {
    fn id_is_available(id: T::PetitionId) -> bool {
        <PetitionStates<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<T::PetitionId> for Module<T> {
    fn generate_unique_id() -> T::PetitionId {
        let mut petition_id_nonce = <PetitionIdNonce<T>>::get() + 1u32.into();
        while !Self::id_is_available(petition_id_nonce) {
            petition_id_nonce += 1u32.into();
        }
        <PetitionIdNonce<T>>::put(petition_id_nonce);
        petition_id_nonce
    }
}

impl<T: Trait> GetVoteOutcome<T::PetitionId> for Module<T> {
    type Outcome = PetitionOutcome;

    fn get_vote_outcome(vote_id: T::PetitionId) -> Result<Self::Outcome, DispatchError> {
        // get petition state
        let petition_state = <PetitionStates<T>>::get(vote_id)
            .ok_or(Error::<T>::CannotGetStatusIfPetitionStateDNE)?;
        //lazily updates storage if outcome changes and returns outcome
        if let Some(new_outcome) = Self::check_petition_outcome(petition_state.clone()) {
            let new_petition_state = petition_state.set_outcome(new_outcome);
            <PetitionStates<T>>::insert(vote_id, new_petition_state);
            Ok(new_outcome)
        } else {
            // outcome didn't change => no changes to storage
            Ok(petition_state.outcome())
        }
    }
}

impl<T: Trait> OpenPetition<T::PetitionId, T::OrgId, T::IpfsReference, T::BlockNumber>
    for Module<T>
{
    fn open_petition(
        organization: T::OrgId,
        topic: Option<T::IpfsReference>,
        required_support: u32,
        vetos_to_reject: u32,
        duration: Option<T::BlockNumber>,
    ) -> Result<T::PetitionId, DispatchError> {
        let total_electorate = <org::Module<T>>::get_size_of_group(organization);
        let ends: Option<T::BlockNumber> = if let Some(time_after_now) = duration {
            let current_time = <frame_system::Module<T>>::block_number();
            Some(current_time + time_after_now)
        } else {
            None
        };
        // returns an error if total_electorate < required_support || total_electorate < vetos_to_reject
        let new_petition_state = PetitionState::new(
            topic,
            organization,
            required_support,
            vetos_to_reject,
            total_electorate,
            ends,
        )
        .ok_or(Error::<T>::PetitionDoesNotSatisfyCreationConstraints)?;
        // insert petition state
        let petition_id = Self::generate_unique_id();
        <PetitionStates<T>>::insert(petition_id, new_petition_state);
        Ok(petition_id)
    }
    /// Why is this separate from `open_petition`? because we only have context for total_electorate inside this method,
    /// not outside of it so we can't just pass total_electorate into `open_petition`
    fn open_unanimous_approval_petition(
        organization: T::OrgId,
        topic: Option<T::IpfsReference>,
        vetos_to_reject: u32,
        duration: Option<T::BlockNumber>,
    ) -> Result<T::PetitionId, DispatchError> {
        // get the total size of the electorate
        let total_electorate = <org::Module<T>>::get_size_of_group(organization);
        let ends: Option<T::BlockNumber> = if let Some(time_after_now) = duration {
            let current_time = <frame_system::Module<T>>::block_number();
            Some(current_time + time_after_now)
        } else {
            None
        };
        let new_petition_state = PetitionState::new(
            topic,
            organization,
            total_electorate, // required_support == total electorate
            vetos_to_reject,
            total_electorate,
            ends,
        )
        .ok_or(Error::<T>::PetitionDoesNotSatisfyCreationConstraints)?;
        // insert petition state
        let petition_id = Self::generate_unique_id();
        <PetitionStates<T>>::insert(petition_id, new_petition_state);
        Ok(petition_id)
    }
}

impl<T: Trait>
    SignPetition<T::PetitionId, T::AccountId, T::IpfsReference, PetitionView<T::IpfsReference>>
    for Module<T>
{
    type Petition = PetitionState<T::OrgId, T::IpfsReference, T::BlockNumber>;
    fn check_petition_outcome(petition_state: Self::Petition) -> Option<Self::Outcome> {
        // check if rejected
        let current_outcome = petition_state.outcome();
        let current_time = <frame_system::Module<T>>::block_number();
        let awaiting_expiry = if let Some(end_time) = petition_state.ends() {
            end_time > current_time
        } else {
            false
        };
        // TODO: rewrite this to be less code, keep storage update lazy though
        if petition_state.rejected() && awaiting_expiry {
            if current_outcome == PetitionOutcome::RejectedButWaitingForTimeToExpire {
                None
            } else {
                Some(PetitionOutcome::RejectedButWaitingForTimeToExpire)
            }
        } else if petition_state.rejected() && !awaiting_expiry {
            if current_outcome == PetitionOutcome::Rejected {
                None
            } else {
                Some(PetitionOutcome::Rejected)
            }
        } else if petition_state.approved() && awaiting_expiry {
            if current_outcome == PetitionOutcome::ApprovedButWaitingForTimeToExpire {
                None
            } else {
                Some(PetitionOutcome::ApprovedButWaitingForTimeToExpire)
            }
        } else if petition_state.approved() && !awaiting_expiry {
            if current_outcome == PetitionOutcome::Approved {
                None
            } else {
                Some(PetitionOutcome::Approved)
            }
        } else {
            if current_outcome == PetitionOutcome::VotingWithNoOutcomeYet {
                None
            } else {
                Some(PetitionOutcome::VotingWithNoOutcomeYet)
            }
        }
    }
    fn sign_petition(
        petition_id: T::PetitionId,
        signer: T::AccountId,
        view: PetitionView<T::IpfsReference>,
    ) -> Result<Self::Outcome, DispatchError> {
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(petition_id)
            .ok_or(Error::<T>::CannotSignIfPetitionStateDNE)?;
        let authentication: bool =
            <org::Module<T>>::is_member_of_group(petition_state.voter_group(), &signer);
        ensure!(
            authentication,
            Error::<T>::CantSignBecauseAccountNotFoundInShareGroup
        );
        // calculate new petition state based on a change in views
        let new_petition_state =
            if let Some(current_signature) = <SignatureLogger<T>>::get(petition_id, &signer) {
                match (current_signature.view(), view.clone()) {
                    // instead of veto_to_freeze
                    (PetitionView::Assent(_), PetitionView::Veto(_)) => {
                        let ps1 = petition_state.revoke_assent();
                        ps1.add_veto()
                    }
                    (PetitionView::Veto(_), PetitionView::Assent(_)) => {
                        let ps1 = petition_state.revoke_veto();
                        ps1.add_assent()
                    }
                    // no comment or the same view has no impact on petition state
                    // TODO: don't insert this into storage again, it's a waste
                    _ => petition_state,
                }
            } else {
                petition_state.apply(view.clone())
            };
        let petition_outcome =
            if let Some(new_outcome) = Self::check_petition_outcome(new_petition_state.clone()) {
                let final_petition_state = new_petition_state.set_outcome(new_outcome);
                <PetitionStates<T>>::insert(petition_id, final_petition_state);
                new_outcome
            } else {
                let outcome = new_petition_state.outcome();
                <PetitionStates<T>>::insert(petition_id, new_petition_state);
                outcome
            };
        let new_signature = PetitionSignature::new(signer.clone(), view);
        <SignatureLogger<T>>::insert(petition_id, &signer, new_signature);
        Ok(petition_outcome)
    }
}

impl<T: Trait>
    UpdatePetition<T::PetitionId, T::AccountId, T::IpfsReference, PetitionView<T::IpfsReference>>
    for Module<T>
{
    fn update_petition(
        petition_id: T::PetitionId,
        new_topic: T::IpfsReference,
        clear_previous_vote_state: bool,
    ) -> DispatchResult {
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(petition_id)
            .ok_or(Error::<T>::CannotUpdateIfPetitionStateDNE)?;
        // update the petition
        let new_petition =
            petition_state.update_petition_terms(new_topic, clear_previous_vote_state);
        <PetitionStates<T>>::insert(petition_id, new_petition);
        Ok(())
    }
}
