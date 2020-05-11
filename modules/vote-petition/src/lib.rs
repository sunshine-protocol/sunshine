#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions
//! Simple module for collecting signatures from organizational share groups
//! - this is a simple vote machine, similar to `vote-yesno` but without any share-weighted threshold or counting complexity

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure, storage::IterableStorageDoubleMap,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::prelude::*;
use util::{
    petition::{PetitionOutcome, PetitionSignature, PetitionState, PetitionView, VetoContext},
    traits::{
        Apply, Approved, ChainSudoPermissions, ChangeGroupMembership, EmpowerWithVeto,
        GenerateUniqueID, GetFlatShareGroup, GetFullVetoContext, GetGroupSize, GetPetitionStatus,
        GroupMembership, IDIsAvailable, OpenPetition, OrganizationSupervisorPermissions, Rejected,
        RequestChanges, SignPetition, SubGroupSupervisorPermissions, UpdatePetition,
        UpdatePetitionTerms, Vetoed,
    },
    uuid::{UUID2, UUID3},
};

/// Ipfs reference just is a type alias over a vector of bytes
pub type IpfsReference = Vec<u8>;

/// The organization identifier
pub type OrgId = u32;

/// The share group identifier
pub type ShareId = u32;

/// The petition identifier
pub type PetitionId = u32;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Just used for permissions in this module
    type OrgData: GroupMembership<Self::AccountId>
        + ChainSudoPermissions<Self::AccountId>
        + OrganizationSupervisorPermissions<u32, Self::AccountId>;

    /// An instance of `SharesMembership`
    type ShareData: GetGroupSize<GroupId = UUID2>
        + GroupMembership<Self::AccountId, GroupId = UUID2>
        + SubGroupSupervisorPermissions<u32, u32, Self::AccountId>
        + ChangeGroupMembership<Self::AccountId>
        + GetFlatShareGroup<Self::AccountId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
    {
        /// Opener's account, Organization, Share Group, Petition Identifier, bool == IF_VETO_POWER_ENABLED
        NewPetitionStarted(AccountId, OrgId, ShareId, PetitionId, bool),
        UserSignedPetition(OrgId, ShareId, PetitionId, AccountId, PetitionOutcome),
        UserVetoedPetition(OrgId, ShareId, PetitionId, AccountId, Option<PetitionOutcome>),
        /// Opener's account, Petition info, New Petition version
        PetitionUpdated(AccountId, OrgId, ShareId, PetitionId, u32),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        /// Local Auths
        NotAuthorizedToCreatePetition,
        NotAuthorizedToUpdatePetition,
        NotAuthorizedToVetoPetition,
        /// The total electorate is less than the required support or required vetos to freeze
        PetitionDoesNotSatisfyCreationConstraints,
        VetoPowerAllocationRequiresUsingVetoToFreeze,
        MustBeEnoughVetoersToFreezeIfSettingVetoers,
        CantSignBecauseAccountNotFoundInShareGroup,
        /// Signatures are not allowed on petitions that have overcome the veto threshold (because they must be updated/changed anyway)
        PetitionIsFrozenByVetoesSoNoSigningAllowed,
        CannotEmpowerWithVetoIfShareMembershipDNE,
        CannotSignIfPetitionStateDNE,
        CannotVetoIfPetitionStateDNE,
        CannotUnVetoIfPetitionStateDNE,
        AlreadyAssentedToPetition,
        CannotUpdateIfPetitionStateDNE,
        CannotGetStatusIfPetitionStateDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as VoteYesNo {
        /// PetitionId storage helper for unique id generation, see issue #62
        pub PetitionIdCounter get(fn petition_id_counter): double_map
            hasher(opaque_blake2_256) OrgId,
            hasher(opaque_blake2_256) ShareId  => PetitionId;

        /// The current state of a petition
        pub PetitionStates get(fn petition_states): double_map
            hasher(opaque_blake2_256) UUID2,
            hasher(opaque_blake2_256) PetitionId => Option<PetitionState<IpfsReference, T::BlockNumber>>;

        /// The signatures of participants in the petition
        pub SignatureLogger get(fn signature_logger): double_map
            hasher(opaque_blake2_256) UUID3,
            hasher(opaque_blake2_256) T::AccountId => Option<PetitionSignature<T::AccountId, IpfsReference>>;

        /// Constant-time check to see if a voter can veto in the context of the petition
        /// and what the state/nature of the veto is...
        pub VetoPower get(fn veto_power): double_map
            hasher(blake2_128_concat) UUID3,
            hasher(blake2_128_concat) T::AccountId => Option<VetoContext<IpfsReference>>;

        /// The outcome of a petition
        pub PetitionOutcomes get(fn petition_outcomes): double_map
            hasher(opaque_blake2_256) UUID2,
            hasher(opaque_blake2_256) PetitionId => Option<PetitionOutcome>;
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
            // default the entire group gets voting power in the petition
            share_id: ShareId,
            proposed_petition_id: Option<PetitionId>,
            topic: IpfsReference,
            required_support: u32,
            required_against: Option<u32>,
            ends: Option<T::BlockNumber>,
            //              if None            => no veto power invoked
            // happy path   if Some, then None => entire share group gets veto power
            //              if Some, then Some => wrapped vec gets veto power
            veto_power: Option<Option<Vec<T::AccountId>>>,
            // TODO: add `approval_power` symmetric to `veto_power`
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::check_if_sudo_account(&caller)
                || Self::check_if_organization_supervisor_account(organization, &caller)
                || Self::check_if_organization_share_supervisor_account(organization, share_id, &caller);
            ensure!(authentication, Error::<T>::NotAuthorizedToCreatePetition);

            // generate uuid in module context
            let petition_id: PetitionId = if let Some(proposed_id) = proposed_petition_id {
                let proposed_id = UUID3::new(organization, share_id, proposed_id);
                Self::generate_unique_id(proposed_id).three()
            } else {
                let id_counter = PetitionIdCounter::get(organization, share_id) + 1u32;
                PetitionIdCounter::insert(organization, share_id, id_counter);
                let generated_id = UUID3::new(organization, share_id, id_counter);
                Self::generate_unique_id(generated_id).three()
            };
            // this is an optional branch only required if we're using veto power
            let mut veto_power_endowed = false;
            if let Some(veto_power_allocation) = veto_power {
                Self::empower_with_veto(
                    organization,
                    share_id,
                    petition_id,
                    veto_power_allocation,
                )?;
                veto_power_endowed = true;
            }
            // create and open the petition
            Self::open_petition(
                organization,
                share_id,
                petition_id,
                topic,
                required_support,
                required_against,
                ends,
            )?;
            Self::deposit_event(RawEvent::NewPetitionStarted(caller, organization, share_id, petition_id, veto_power_endowed));
            Ok(())
        }

        #[weight = 0]
        pub fn direct_sign_petition(
            origin,
            organization: OrgId,
            share_id: ShareId,
            petition_id: PetitionId,
            view: PetitionView,
            justification: IpfsReference,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // this implies that only members of the share group are in the main signature electorate
            let authentication: bool = Self::check_if_account_is_in_share_group(organization, share_id, &signer);
            ensure!(authentication, Error::<T>::CantSignBecauseAccountNotFoundInShareGroup);
            // sign petition with API
            let outcome = Self::sign_petition(
                organization,
                share_id,
                petition_id,
                signer.clone(),
                view,
                justification
            )?;
            Self::deposit_event(RawEvent::UserSignedPetition(organization, share_id, petition_id, signer, outcome));
            Ok(())
        }

        #[weight = 0]
        pub fn direct_veto_to_request_changes(
            origin,
            organization: OrgId,
            share_id: ShareId,
            petition_id: PetitionId,
            justification: IpfsReference,
        ) -> DispatchResult {
            let signer = ensure_signed(origin)?;
            // veto petition with API
            let outcome = Self::request_changes(
                organization,
                share_id,
                petition_id,
                signer.clone(),
                justification
            )?;
            Self::deposit_event(RawEvent::UserVetoedPetition(organization, share_id, petition_id, signer, outcome));
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

            let new_version = Self::update_petition(organization, share_id, petition_id, new_topic)?;
            Self::deposit_event(RawEvent::PetitionUpdated(caller, organization, share_id, petition_id, new_version));
            Ok(())
        }
        // More TODO:
        // approve_petition
        // proxy_sign_petition
        // proxy_veto_petition
        // NOTE: batch methods only exposed in the proxy API with explicit permissions
        // proxy_batch_sign_petition
        // proxy_batch_veto_petition
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

impl<T: Trait> IDIsAvailable<UUID3> for Module<T> {
    fn id_is_available(id: UUID3) -> bool {
        None == <PetitionStates<T>>::get(id.one_two(), id.three())
    }
}

impl<T: Trait> GenerateUniqueID<UUID3> for Module<T> {
    fn generate_unique_id(proposed_id: UUID3) -> UUID3 {
        let organization = proposed_id.one();
        let share_id = proposed_id.two();
        let one_two = proposed_id.one_two();
        if !Self::id_is_available(proposed_id) || proposed_id.three() == 0u32 {
            let mut id_counter = PetitionIdCounter::get(organization, share_id);
            while <PetitionStates<T>>::get(one_two, id_counter).is_some() || id_counter == 0u32 {
                // TODO: add overflow check here
                id_counter += 1u32;
            }
            PetitionIdCounter::insert(organization, share_id, id_counter);
            UUID3::new(organization, share_id, id_counter)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> GetPetitionStatus for Module<T> {
    type Status = u32; // says approvals gotten and outcome, including veto context?

    // TODO: update this for what is necessary in bounty when calling in ie bounty
    fn get_petition_status(
        organization: OrgId,
        share_id: ShareId,
        petition_id: PetitionId,
    ) -> Result<Self::Status, DispatchError> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
            .ok_or(Error::<T>::CannotGetStatusIfPetitionStateDNE)?;
        Ok(petition_state.version())
    }
}

impl<T: Trait> EmpowerWithVeto<T::AccountId> for Module<T> {
    fn get_those_empowered_with_veto(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<T::AccountId>> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        let empowered_vetoers = <VetoPower<T>>::iter()
            .filter(|(uuidtwo, _, _)| uuidtwo == &prefix)
            .map(|(_, account, _)| account)
            .collect::<Vec<_>>();
        if empowered_vetoers.is_empty() {
            None
        } else {
            Some(empowered_vetoers)
        }
    }
    fn get_those_who_invoked_veto(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<T::AccountId>> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        let vetoers_who_invoked = <VetoPower<T>>::iter()
            .filter(|(uuidthree, _, context)| uuidthree == &prefix && context.invoked())
            .map(|(_, account, _)| account)
            .collect::<Vec<_>>();
        if vetoers_who_invoked.is_empty() {
            None
        } else {
            Some(vetoers_who_invoked)
        }
    }
    fn empower_with_veto(
        organization: OrgId,
        share_id: ShareId,
        petition_id: PetitionId,
        // if none, give it to everyone in the share_id group
        accounts: Option<Vec<T::AccountId>>,
    ) -> DispatchResult {
        let veto_power_recipients = if let Some(accounts_endowed_with_veto) = accounts {
            // NOTE: there is not check that the members in this group are in the share_id group `=>` outside members CAN be endowed with veto power unless some check against this is implemented
            accounts_endowed_with_veto
        } else {
            // return all the accounts in the share group
            if let Some(entire_share_group) =
                <<T as Trait>::ShareData as GetFlatShareGroup<
                    <T as frame_system::Trait>::AccountId,
                >>::get_organization_share_group(organization, share_id)
            {
                entire_share_group
            } else {
                return Err(Error::<T>::CannotEmpowerWithVetoIfShareMembershipDNE.into());
            }
        };
        let prefix = UUID3::new(organization, share_id, petition_id);
        // issue them all a power check
        veto_power_recipients.into_iter().for_each(|recipient| {
            // set the constant time check for if voter can invoke veto
            <VetoPower<T>>::insert(prefix, recipient, VetoContext::default());
        });
        Ok(())
    }
}

impl<T: Trait> OpenPetition<IpfsReference, T::BlockNumber> for Module<T> {
    fn open_petition(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        topic: IpfsReference,
        required_support: u32,
        require_against: Option<u32>,
        ends: Option<T::BlockNumber>,
    ) -> DispatchResult {
        // get the total size of the electorate
        let prefix = UUID2::new(organization, share_id);
        let total_electorate = <<T as Trait>::ShareData as GetGroupSize>::get_size_of_group(prefix);
        // returns an error if total_electorate < required_support || total_electorate < required_vetos_to_freeze
        let new_petition_state = PetitionState::new(
            topic,
            required_support,
            require_against,
            total_electorate,
            ends,
        )
        .ok_or(Error::<T>::PetitionDoesNotSatisfyCreationConstraints)?;
        // insert petition state
        let prefix = UUID3::new(organization, share_id, petition_id);
        <PetitionStates<T>>::insert(prefix.one_two(), petition_id, new_petition_state);
        // set new petition outcome for new petition state
        PetitionOutcomes::insert(
            prefix.one_two(),
            petition_id,
            PetitionOutcome::VoteWithNoOutcomeYet,
        );
        Ok(())
    }
}

impl<T: Trait> GetFullVetoContext<T::AccountId> for Module<T> {
    type VetoContext = VetoContext<IpfsReference>;
    fn get_full_veto_context(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<(T::AccountId, Self::VetoContext)>> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        let full_veto_context = <VetoPower<T>>::iter()
            .filter(|(uuidthree, _, _)| uuidthree == &prefix)
            .map(|(_, account, context)| (account, context))
            .collect::<Vec<_>>();
        if full_veto_context.is_empty() {
            None
        } else {
            Some(full_veto_context)
        }
    }
}

impl<T: Trait> SignPetition<T::AccountId, IpfsReference> for Module<T> {
    type Petition = PetitionState<IpfsReference, T::BlockNumber>;
    type SignerView = PetitionView;
    type Outcome = PetitionOutcome;
    fn sign_petition(
        organization: OrgId,
        share_id: ShareId,
        petition_id: PetitionId,
        signer: T::AccountId,
        view: PetitionView,
        justification: IpfsReference,
    ) -> Result<Self::Outcome, DispatchError> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
            .ok_or(Error::<T>::CannotSignIfPetitionStateDNE)?;
        // check if its frozen, if so, cannot vote, return error
        ensure!(
            !petition_state.vetoed(),
            Error::<T>::PetitionIsFrozenByVetoesSoNoSigningAllowed
        );
        // calculate new petition state based on a change in views
        let new_petition_state =
            if let Some(current_signature) = <SignatureLogger<T>>::get(prefix, &signer) {
                match (current_signature.view(), view) {
                    (PetitionView::Assent, PetitionView::Dissent) => {
                        let ps1 = petition_state.revoke_assent();
                        ps1.add_dissent()
                    }
                    (PetitionView::Dissent, PetitionView::Assent) => {
                        let ps1 = petition_state.revoke_dissent();
                        ps1.add_assent()
                    }
                    // no comment or the same view has no impact on petition state
                    _ => petition_state,
                }
            } else {
                petition_state.apply(view)
            };
        // insert new petition state
        <PetitionStates<T>>::insert(prefix.one_two(), petition_id, new_petition_state.clone());
        let new_signature = PetitionSignature::new(signer.clone(), view, justification);
        <SignatureLogger<T>>::insert(prefix, &signer, new_signature);
        // get new outcome, TODO: save a clone by doing this before inserting petition state
        let current_block: T::BlockNumber = <frame_system::Module<T>>::block_number();
        let past_ends: bool = if let Some(end_time) = new_petition_state.ends() {
            end_time <= current_block
        } else {
            true
        };
        // define new outcome based on any changes to the petition state
        let new_outcome =
            if new_petition_state.approved() && past_ends && !new_petition_state.rejected() {
                PetitionOutcome::Approved
            } else if new_petition_state.rejected() && past_ends && new_petition_state.approved() {
                PetitionOutcome::Rejected
            } else if new_petition_state.approved() && !past_ends {
                PetitionOutcome::ApprovedButWaiting
            } else if new_petition_state.rejected() && !past_ends {
                PetitionOutcome::RejectedButWaiting
            } else {
                // default
                PetitionOutcome::VoteWithNoOutcomeYet
            };
        // set new outcome
        PetitionOutcomes::insert(prefix.one_two(), petition_id, new_outcome);
        // return new outcome
        Ok(new_outcome)
    }
}

impl<T: Trait> RequestChanges<T::AccountId, IpfsReference> for Module<T> {
    fn request_changes(
        organization: OrgId,
        share_id: ShareId,
        petition_id: PetitionId,
        signer: T::AccountId,
        justification: IpfsReference,
    ) -> Result<Option<Self::Outcome>, DispatchError> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
            .ok_or(Error::<T>::CannotVetoIfPetitionStateDNE)?;
        let new_outcome: Option<PetitionOutcome> = if <VetoPower<T>>::get(prefix, &signer).is_some()
        {
            // overwrites any existing veto by default, this could be better
            let new_veto = VetoContext::requested_changes(justification);
            <VetoPower<T>>::insert(prefix, &signer, new_veto);
            let new_petition_state = petition_state.veto_to_freeze();
            <PetitionStates<T>>::insert(prefix.one_two(), petition_id, new_petition_state);
            PetitionOutcomes::insert(prefix.one_two(), petition_id, PetitionOutcome::FrozenByVeto);
            Some(PetitionOutcome::FrozenByVeto)
        } else {
            return Err(Error::<T>::NotAuthorizedToVetoPetition.into());
        };
        // return outcome
        Ok(new_outcome)
    }
    fn accept_changes(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        signer: T::AccountId,
    ) -> Result<Option<Self::Outcome>, DispatchError> {
        let prefix = UUID3::new(organization, share_id, petition_id);
        // get the petition state
        let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
            .ok_or(Error::<T>::CannotUnVetoIfPetitionStateDNE)?;
        let new_outcome: Option<PetitionOutcome> = if <VetoPower<T>>::get(prefix, &signer).is_some()
        {
            // overwrites any existing veto by default, this could be better
            let revoke_veto = VetoContext::accept_changes();
            <VetoPower<T>>::insert(prefix, &signer, revoke_veto);
            if Self::get_those_who_invoked_veto(organization, share_id, petition_id).is_none() {
                // only call revoke veto to unfreeze() if there are no vetos left after we inserted the revocation
                let new_petition_state = petition_state.revoke_veto_to_unfreeze();
                <PetitionStates<T>>::insert(
                    prefix.one_two(),
                    petition_id,
                    new_petition_state.clone(),
                );
                let current_block: T::BlockNumber = <frame_system::Module<T>>::block_number();
                let past_ends: bool = if let Some(end_time) = new_petition_state.ends() {
                    end_time <= current_block
                } else {
                    true
                };
                // define new outcome based on any changes to the petition state
                let new_new_outcome =
                    if new_petition_state.approved() && past_ends && !new_petition_state.rejected()
                    {
                        PetitionOutcome::Approved
                    } else if new_petition_state.rejected()
                        && past_ends
                        && new_petition_state.approved()
                    {
                        PetitionOutcome::Rejected
                    } else if new_petition_state.approved() && !past_ends {
                        PetitionOutcome::ApprovedButWaiting
                    } else if new_petition_state.rejected() && !past_ends {
                        PetitionOutcome::RejectedButWaiting
                    } else {
                        // default
                        PetitionOutcome::VoteWithNoOutcomeYet
                    };
                PetitionOutcomes::insert(prefix.one_two(), petition_id, new_new_outcome);
                Some(new_new_outcome)
            } else {
                None // no change in the outcome so this is equivalent to Some(PetitionOutcome::FrozenByVeto)
            }
        } else {
            return Err(Error::<T>::NotAuthorizedToVetoPetition.into());
        };
        // return outcome
        Ok(new_outcome)
    }
}

impl<T: Trait> UpdatePetition<T::AccountId, IpfsReference> for Module<T> {
    fn update_petition(
        organization: OrgId,
        share_id: ShareId,
        petition_id: PetitionId,
        new_topic: IpfsReference,
    ) -> Result<u32, DispatchError> {
        let prefix = UUID3::new(organization, share_id, petition_id);

        // get the petition state
        let petition_state = <PetitionStates<T>>::get(prefix.one_two(), petition_id)
            .ok_or(Error::<T>::CannotUpdateIfPetitionStateDNE)?;

        // update the petition
        let new_petition = petition_state.update_petition_terms(new_topic);
        // insert updated petition into storage, approval count is reset
        let new_version = new_petition.version();
        <PetitionStates<T>>::insert(prefix.one_two(), petition_id, new_petition);
        Ok(new_version)
    }
}
