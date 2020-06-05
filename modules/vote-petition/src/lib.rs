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

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure}; // storage::IterableStorageDoubleMap
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
use util::{
    petition::{PetitionOutcome, PetitionSignature, PetitionState, PetitionView},
    traits::{
        Apply, Approved, ChainSudoPermissions, ChangeGroupMembership, GenerateUniqueID,
        GetFlatShareGroup, GetGroupSize, GetVoteOutcome, GroupMembership, IDIsAvailable,
        OpenPetition, OrganizationSupervisorPermissions, Rejected, SignPetition,
        SubGroupSupervisorPermissions, UpdatePetition, UpdatePetitionTerms,
    }, // RequestChanges
    uuid::UUID2,
};

/// Ipfs reference just is a type alias over a vector of bytes
pub type IpfsReference = Vec<u8>;

/// The organization identifier
pub type OrgId = u32;
/// The petition identifier
pub type PetitionId = u32;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type IpfsReference; // Codec + Parameter at least

    /// Just used for permissions in this module
    type OrgData: GroupMembership<Self::AccountId>
        + ChainSudoPermissions<Self::AccountId>
        + OrganizationSupervisorPermissions<u32, Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        /// Opener's account, Organization, Share Group, Petition Identifier, bool == IF_VETO_POWER_ENABLED
        NewPetitionStarted(AccountId, OrgId, ShareId, PetitionId),
        UserSignedPetition(PetitionId, AccountId, PetitionView<IpfsReference>, PetitionOutcome),
        /// Opener's account, Petition info, New Petition version
        PetitionUpdated(AccountId, OrgId, ShareId, PetitionId, u32),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Local Auths
        NotAuthorizedToCreatePetition,
        NotAuthorizedToUpdatePetition,
        /// The total electorate is less than the required support or required vetos to freeze
        PetitionDoesNotSatisfyCreationConstraints,
        CantSignBecauseAccountNotFoundInShareGroup,
        CannotSignIfPetitionStateDNE,
        CannotUpdateIfPetitionStateDNE,
        CannotGetStatusIfPetitionStateDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as VotePetition {
        /// For efficient UID generation inside this module
        PetitionIdNonce get(fn petition_id_nonce): PetitionId;
        /// The current state of a petition
        pub PetitionStates get(fn petition_states): map
            hasher(opaque_blake2_256) PetitionId => Option<PetitionState<IpfsReference, T::BlockNumber>>;

        /// The signatures of participants in the petition
        pub SignatureLogger get(fn signature_logger): double_map
            hasher(opaque_blake2_256) PetitionId,
            hasher(opaque_blake2_256) T::AccountId => Option<PetitionSignature<T::AccountId, IpfsReference>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn create_petition(
            origin,
            organization: OrgId,
            share_id: ShareId,
            topic: Option<IpfsReference>,
            required_support: u32,
            required_against: Option<u32>,
            ends: Option<T::BlockNumber>,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller)
                || Self::check_if_organization_share_supervisor_account(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreatePetition);
            // create and open the petition
            let petition_id = Self::open_petition(
                organization,
                share_id,
                topic,
                required_support,
                required_against,
                ends,
            )?;
            Self::deposit_event(RawEvent::NewPetitionStarted(caller, organization, share_id, petition_id));
            Ok(())
        }
        #[weight = 0]
        pub fn direct_sign_petition(
            origin,
            petition_id: PetitionId,
            view: PetitionView<IpfsReference>,
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
        pub fn supervisor_update_petition(
            origin,
            organization: OrgId,
            share_id: ShareId,
            petition_id: PetitionId,
            new_topic: IpfsReference,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller)
                || Self::check_if_organization_share_supervisor_account(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToUpdatePetition);

            // TODO: get the outcome and store it, frozen for the old version

            Self::update_petition(petition_id, new_topic)?;
            // Self::deposit_event(RawEvent::PetitionUpdated(caller, organization, share_id, petition_id));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    // $$$ AUTH CHECKS $$$
    fn check_if_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as ChainSudoPermissions<<T as frame_system::Trait>::AccountId>>::is_sudo_key(who)
    }
    fn check_if_organization_supervisor_account(organization: OrgId, who: &T::AccountId) -> bool {
        <<T as Trait>::OrgData as OrganizationSupervisorPermissions<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::is_organization_supervisor(organization, who)
    }
    fn check_if_organization_share_supervisor_account(
        organization: OrgId,
        share_id: ShareId,
        who: &T::AccountId,
    ) -> bool {
        <<T as Trait>::ShareData as SubGroupSupervisorPermissions<
            u32,
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::is_sub_group_supervisor(organization, share_id, who)
    }
    // fn check_if_account_is_member_in_organization(
    //     organization: OrgId,
    //     account: &T::AccountId,
    // ) -> bool {
    //     <<T as Trait>::OrgData as OrganizationMembership<<T as frame_system::Trait>::AccountId>>::is_member_of_organization(organization, account)
    // }
    fn check_if_account_is_in_share_group(
        organization: OrgId,
        share_id: ShareId,
        account: &T::AccountId,
    ) -> bool {
        let prefix = UUID2::new(organization, share_id);
        <<T as Trait>::ShareData as GroupMembership<<T as frame_system::Trait>::AccountId>>::is_member_of_group(prefix, account)
    }
}

impl<T: Trait> IDIsAvailable<PetitionId> for Module<T> {
    fn id_is_available(id: PetitionId) -> bool {
        <PetitionStates<T>>::get(id).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<PetitionId> for Module<T> {
    fn generate_unique_id() -> PetitionId {
        let mut petition_id_nonce = PetitionIdNonce::get() + 1u32;
        while !Self::id_is_available(petition_id_nonce) {
            petition_id_nonce += 1u32;
        }
        PetitionIdNonce::put(petition_id_nonce);
        petition_id_nonce
    }
}

impl<T: Trait> GetVoteOutcome for Module<T> {
    type VoteId = PetitionId;
    type Outcome = PetitionOutcome; // could be more contextful

    fn get_vote_outcome(vote_id: Self::VoteId) -> Result<Self::Outcome, DispatchError> {
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(vote_id)
            .ok_or(Error::<T>::CannotGetStatusIfPetitionStateDNE)?;
        // TODO: lazy update that only makes insertion if outcome changes
        let outcome_check = Self::check_petition_outcome(petition_state.clone())?;
        let updated_petition_state = petition_state.set_outcome(outcome_check);
        <PetitionStates<T>>::insert(vote_id, updated_petition_state);
        Ok(outcome_check)
    }
}

impl<T: Trait> OpenPetition<IpfsReference, T::BlockNumber> for Module<T> {
    fn open_petition(
        organization: u32,
        share_id: u32,
        topic: Option<IpfsReference>,
        required_support: u32,
        require_against: Option<u32>,
        duration: Option<T::BlockNumber>,
    ) -> Result<Self::VoteId, DispatchError> {
        // get the total size of the electorate
        let prefix = UUID2::new(organization, share_id);
        let total_electorate = <<T as Trait>::ShareData as GetGroupSize>::get_size_of_group(prefix);
        let ends: Option<T::BlockNumber> = if let Some(time_after_now) = duration {
            let current_time = <frame_system::Module<T>>::block_number();
            Some(current_time + time_after_now)
        } else {
            None
        };
        // returns an error if total_electorate < required_support
        let new_petition_state = PetitionState::new(
            topic,
            prefix.into(),
            required_support,
            require_against,
            total_electorate,
            ends,
        )
        .ok_or(Error::<T>::PetitionDoesNotSatisfyCreationConstraints)?;
        // insert petition state
        let petition_id = Self::generate_unique_id();
        <PetitionStates<T>>::insert(petition_id, new_petition_state);
        Ok(petition_id)
    }
    // why do we need this? because we only have context for total_electorate in this method,
    // not outside of it so we can't just pass total_electorate into `open_petition`
    fn open_unanimous_approval_petition(
        organization: u32,
        share_id: u32,
        topic: Option<IpfsReference>,
        duration: Option<T::BlockNumber>,
    ) -> Result<Self::VoteId, DispatchError> {
        // get the total size of the electorate
        let prefix = UUID2::new(organization, share_id);
        let total_electorate = <<T as Trait>::ShareData as GetGroupSize>::get_size_of_group(prefix);
        let ends: Option<T::BlockNumber> = if let Some(time_after_now) = duration {
            let current_time = <frame_system::Module<T>>::block_number();
            Some(current_time + time_after_now)
        } else {
            None
        };
        // returns an error if total_electorate < required_support || total_electorate < required_vetos_to_freeze
        let new_petition_state = PetitionState::new(
            topic,
            prefix.into(),
            total_electorate,
            None,
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

impl<T: Trait> SignPetition<T::AccountId, IpfsReference> for Module<T> {
    type Petition = PetitionState<IpfsReference, T::BlockNumber>;
    type SignerView = PetitionView<IpfsReference>;
    fn check_petition_outcome(petition: Self::Petition) -> Result<Self::Outcome, DispatchError> {
        let current_block: T::BlockNumber = <frame_system::Module<T>>::block_number();
        let past_ends: bool = if let Some(end_time) = petition.ends() {
            end_time <= current_block
        } else {
            true
        };
        // define new outcome based on any changes to the petition state
        let new_outcome = if petition.approved() && past_ends && !petition.rejected() {
            PetitionOutcome::Approved
        } else if petition.rejected() && past_ends && petition.approved() {
            PetitionOutcome::Rejected
        } else if petition.approved() && !past_ends {
            PetitionOutcome::ApprovedButWaitingForTimeToExpire
        } else if petition.rejected() && !past_ends {
            PetitionOutcome::RejectedButWaitingForTimeToExpire
        } else {
            // default
            PetitionOutcome::VotingWithNoOutcomeYet
        };
        Ok(new_outcome)
    }
    fn sign_petition(
        petition_id: PetitionId,
        signer: T::AccountId,
        view: PetitionView<IpfsReference>,
    ) -> Result<Self::Outcome, DispatchError> {
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(petition_id)
            .ok_or(Error::<T>::CannotSignIfPetitionStateDNE)?;
        // check that the signer can sign the petition
        let org_share = petition_state.voter_id_reqs();
        let authentication: bool =
            Self::check_if_account_is_in_share_group(org_share.0, org_share.1, &signer);
        ensure!(
            authentication,
            Error::<T>::CantSignBecauseAccountNotFoundInShareGroup
        );
        // calculate new petition state based on a change in views
        let new_petition_state =
            if let Some(current_signature) = <SignatureLogger<T>>::get(petition_id, &signer) {
                match (current_signature.view(), view.clone()) {
                    (PetitionView::Assent(_), PetitionView::Dissent(_)) => {
                        let ps1 = petition_state.revoke_assent();
                        ps1.add_dissent()
                    }
                    // instead of veto_to_freeze
                    (PetitionView::Assent(_), PetitionView::Veto(_)) => {
                        let ps1 = petition_state.revoke_assent();
                        ps1.add_veto()
                    }
                    (PetitionView::Dissent(_), PetitionView::Assent(_)) => {
                        let ps1 = petition_state.revoke_dissent();
                        ps1.add_assent()
                    }
                    (PetitionView::Dissent(_), PetitionView::Veto(_)) => {
                        let ps1 = petition_state.revoke_dissent();
                        ps1.add_veto()
                    }
                    (PetitionView::Veto(_), PetitionView::Assent(_)) => {
                        let ps1 = petition_state.revoke_veto();
                        ps1.add_assent()
                    }
                    (PetitionView::Veto(_), PetitionView::Dissent(_)) => {
                        let ps1 = petition_state.revoke_veto();
                        ps1.add_dissent()
                    }
                    // no comment or the same view has no impact on petition state
                    _ => petition_state,
                }
            } else {
                petition_state.apply(view.clone())
            };
        let petition_outcome = Self::check_petition_outcome(new_petition_state.clone())?;
        let updated_petition_state = new_petition_state.set_outcome(petition_outcome);
        <PetitionStates<T>>::insert(petition_id, updated_petition_state);
        let new_signature = PetitionSignature::new(signer.clone(), view);
        <SignatureLogger<T>>::insert(petition_id, &signer, new_signature);
        Ok(petition_outcome)
    }
}

// impl<T: Trait> RequestChanges<T::AccountId, IpfsReference> for Module<T> {
//     fn request_changes(
//         petition_id: PetitionId,
//         signer: T::AccountId,
//         justification: IpfsReference,
//     ) -> Result<Option<Self::Outcome>, DispatchError> {
//         // get the petition state
//         let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
//             .ok_or(Error::<T>::CannotVetoIfPetitionStateDNE)?;
//         let new_outcome: Option<PetitionOutcome> = if <VetoPower<T>>::get(prefix, &signer).is_some()
//         {
//             // overwrites any existing veto by default, this could be better
//             let new_veto = VetoContext::requested_changes(justification);
//             <VetoPower<T>>::insert(prefix, &signer, new_veto);
//             let new_petition_state = petition_state.veto_to_freeze();
//             <PetitionStates<T>>::insert(prefix.one_two(), petition_id, new_petition_state);
//             PetitionOutcomes::insert(prefix.one_two(), petition_id, PetitionOutcome::FrozenByVeto);
//             Some(PetitionOutcome::FrozenByVeto)
//         } else {
//             return Err(Error::<T>::NotAuthorizedToVetoPetition.into());
//         };
//         // return outcome
//         Ok(new_outcome)
//     }
//     fn accept_changes(
//         petition_id: u32,
//         signer: T::AccountId,
//     ) -> Result<Option<Self::Outcome>, DispatchError> {
//         let prefix = UUID3::new(organization, share_id, petition_id);
//         // get the petition state
//         let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
//             .ok_or(Error::<T>::CannotUnVetoIfPetitionStateDNE)?;
//         let new_outcome: Option<PetitionOutcome> = if <VetoPower<T>>::get(prefix, &signer).is_some()
//         {
//             // overwrites any existing veto by default, this could be better
//             let revoke_veto = VetoContext::accept_changes();
//             <VetoPower<T>>::insert(prefix, &signer, revoke_veto);
//             if Self::get_those_who_invoked_veto(organization, share_id, petition_id).is_none() {
//                 // only call revoke veto to unfreeze() if there are no vetos left after we inserted the revocation
//                 let new_petition_state = petition_state.revoke_veto_to_unfreeze();
//                 <PetitionStates<T>>::insert(
//                     prefix.one_two(),
//                     petition_id,
//                     new_petition_state.clone(),
//                 );
//                 let current_block: T::BlockNumber = <frame_system::Module<T>>::block_number();
//                 let past_ends: bool = if let Some(end_time) = new_petition_state.ends() {
//                     end_time <= current_block
//                 } else {
//                     true
//                 };
//                 // define new outcome based on any changes to the petition state
//                 let new_new_outcome =
//                     if new_petition_state.approved() && past_ends && !new_petition_state.rejected()
//                     {
//                         PetitionOutcome::Approved
//                     } else if new_petition_state.rejected()
//                         && past_ends
//                         && new_petition_state.approved()
//                     {
//                         PetitionOutcome::Rejected
//                     } else if new_petition_state.approved() && !past_ends {
//                         PetitionOutcome::ApprovedButWaiting
//                     } else if new_petition_state.rejected() && !past_ends {
//                         PetitionOutcome::RejectedButWaiting
//                     } else {
//                         // default
//                         PetitionOutcome::VotingWithNoOutcomeYet
//                     };
//                 PetitionOutcomes::insert(prefix.one_two(), petition_id, new_new_outcome);
//                 Some(new_new_outcome)
//             } else {
//                 None // no change in the outcome so this is equivalent to Some(PetitionOutcome::FrozenByVeto)
//             }
//         } else {
//             return Err(Error::<T>::NotAuthorizedToVetoPetition.into());
//         };
//         // return outcome
//         Ok(new_outcome)
//     }
// }

impl<T: Trait> UpdatePetition<T::AccountId, IpfsReference> for Module<T> {
    fn update_petition(petition_id: PetitionId, new_topic: IpfsReference) -> DispatchResult {
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(petition_id)
            .ok_or(Error::<T>::CannotUpdateIfPetitionStateDNE)?;
        // update the petition
        let new_petition = petition_state.update_petition_terms(new_topic);
        <PetitionStates<T>>::insert(petition_id, new_petition);
        Ok(())
    }
}
