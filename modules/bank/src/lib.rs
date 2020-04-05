#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Get};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::Zero,
    DispatchError,
    DispatchResult,
    Permill, // ModuleId //add to Organization struct once substrate#5149 is addressed
};
use sp_std::prelude::*;
use util::{
    organization::Organization,
    proposal::{ProposalIndex, ProposalStage, ProposalType, SimplePollingOutcome},
    traits::{
        Approved,
        GenerateUniqueID,
        GetCurrentVoteIdentifiers,
        GetVoteOutcome,
        GroupMembership,
        IDIsAvailable,
        LockableProfile,
        OpenVote,
        PollActiveProposal,
        ReservableProfile,
        ScheduleVoteSequence,
        SetDefaultShareApprovalOrder,
        SetDefaultShareIdThreshold,
        ShareBank,
        ShareRegistration,
        SudoKeyManagement,
        SupervisorKeyManagement,
        VoteOnProposal, // ScheduleCustomVoteSequence
        VoteScheduleBuilder,
        VoteScheduler,
    },
    uuid::{OrgSharePrefixKey, OrgShareVotePrefixKey},
    voteyesno::{ScheduledVote, ThresholdConfig, VoteSchedule},
};

/// The organization identifier type
pub type OrgId<T> =
    <<T as Trait>::ShareData as ShareRegistration<<T as frame_system::Trait>::AccountId>>::OrgId;

/// The share identifier type
pub type ShareId<T> =
    <<T as Trait>::ShareData as ShareRegistration<<T as frame_system::Trait>::AccountId>>::ShareId;

/// The binary vote identifier type
pub type VoteId<T> =
    <<T as Trait>::BinaryVoteMachine as GetVoteOutcome<OrgId<T>, ShareId<T>>>::VoteId;

/// The share identifier type for binary votes (associated with the Trait's binary vote machine)
/// The shares type that is converted into signal for each instance of this module
pub type SharesOf<T> =
    <<T as Trait>::ShareData as ShareRegistration<<T as frame_system::Trait>::AccountId>>::Shares;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// An instance of the shares module
    type ShareData: GroupMembership<Self::AccountId>
        + ShareRegistration<Self::AccountId>
        + ReservableProfile<Self::AccountId>
        + LockableProfile<Self::AccountId>
        + ShareBank<Self::AccountId>
        + IDIsAvailable<OrgSharePrefixKey<OrgId<Self>, ShareId<Self>>>
        + GenerateUniqueID<OrgSharePrefixKey<OrgId<Self>, ShareId<Self>>>;

    /// The `vote-yesno` module instance
    type BinaryVoteMachine: GetVoteOutcome<OrgId<Self>, ShareId<Self>>
        + OpenVote<OrgId<Self>, ShareId<Self>, Self::AccountId, Permill>
        + VoteOnProposal<OrgId<Self>, ShareId<Self>, Self::AccountId, Permill>
        + IDIsAvailable<OrgShareVotePrefixKey<OrgId<Self>, ShareId<Self>, VoteId<Self>>>
        + GenerateUniqueID<OrgShareVotePrefixKey<OrgId<Self>, ShareId<Self>, VoteId<Self>>>;

    /// The number of blocks between the polling of all active proposals for all organizations (see issue #84)
    type PollingFrequency: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        OrgId = OrgId<T>,
        ShareId = ShareId<T>,
        VoteId = VoteId<T>,
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::Hash,
        <T as frame_system::Trait>::BlockNumber,
    {
        /// The account that summoned, the organization id, and the admin id within the organization
        NewOrganizationRegistered(AccountId, OrgId, ShareId),
        /// The organization's constitution was updated to the hash by the account
        ConstitutionUpdated(AccountId, OrgId, Hash),
        /// The given account registered a new share type for the organization
        NewShareTypeRegisteredForOrganization(AccountId, OrgId, ShareId),
        /// The given account created a vote for the share group
        SingleVoteCreatedForShareGroup(AccountId, OrgId, ShareId, VoteId, BlockNumber),
        /// The account in question dispatched a proposal with this information, it can be used to query the state for more nuanced updates
        ProposalDispatchedToVote(AccountId, OrgId, ProposalIndex, VoteId, BlockNumber),
        /// The account set this default threshold, step 1 of the default build method call sequence
        NewOrgShareProposalThresholdSet(AccountId, OrgId, ShareId, ProposalType, Permill, Permill),
        /// The account in question changed the default share approval order storage item
        MostBasicVoteRequirementSet(AccountId, OrgId, ProposalType),
        /// The vote schedule for the organization, proposal_id has proceeded to the next vote
        VoteScheduleProgress(OrgId, ProposalIndex, VoteId, VoteId),
        /// The proposal in question passed (<=> all votes in vote schedule passed)
        ProposalPassed(OrgId, ProposalIndex),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        MustScheduleAtLeastOneVote,
        /// The account that requested the change to the organization's
        /// constitution does not have controller privileges
        NotAuthorizedToChangeConstitution,
        NotAuthorizedToSetVoteRequirements,
        NotAuthorizedToSetOrgShareIdProposalTypeThreshold,
        OnlySudoKeyCanSummonForNow,
        /// this occurs in the context of updating the value constitution
        /// but is equivalent to the `NoRegisteredOrganizationWithIDProvided` (TODO: enforce invariant)
        NoExistingValueConstitution,
        NoRegisteredOrganizationWithIDProvided,
        DefaultThresholdForShareIdNotSet,
        /// Vote sequence was attempted to be scheduled but must have failed because no votes were scheduled
        NoVoteStarted,
        /// Polling was not possible because the org_id, proposal_index doesn't have a vote schedule
        NoScheduledVoteSequence,
        /// This module operates on two layers of defaults, best to document them today to not risk getting confused again in the future
        NoDefaultSetAndNoShareVoteScheduleProvided,
        UnAuthorizedSwapSudoRequest,
        UnAuthorizedRequestToSwapSupervisor,
        /// This error means a swap was attempted but there is no SudoKey in storage
        NoExistingSudoKeySoChainBricked,
        // REMOVE after development is over
        DevelopmentPlaceHolderError,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// The account that can set all the organization supervisors, should be replaced by committee-based governance
        SudoKey build(|config: &GenesisConfig<T>| Some(config.omnipotent_key.clone())): Option<T::AccountId>;
        /// The account that can change the constitution
        /// - by default the summoner
        OrganizationSupervisor get(fn organization_supervisor):
            map hasher(blake2_256) OrgId<T> => Option<T::AccountId>;

        /// The identity generator nonce for OrgId<T>
        OrganizationIdentityNonce get(fn organization_identity_nonce): OrgId<T>;

        /// The number of organizations registered on-chain
        OrganizationCount get(fn organization_count): u32;

        OrganizationState get(fn organization_state): map
            hasher(blake2_256) OrgId<T> => Option<Organization<ShareId<T>>>;

        /// This constitution should be used to guide voting
        /// - it is also the main anchor for any organization and therefore defines registration
        ValueConstitution get(fn value_constitution):
            map hasher(blake2_256) OrgId<T> => Option<T::Hash>;

        /// The total number of proposals for an organization, also used as the nonce
        ProposalCount get(fn proposal_count): map hasher(blake2_256) OrgId<T> => ProposalIndex;

        /// Every time a proposal is added, it should be added to this
        /// - this storage item is used in on_finalize to poll all active proposals (see `PollActiveProposal` trait)
        AllActiveProposalKeys get(fn all_active_proposal_keys): Vec<(OrgId<T>, ProposalIndex)>;

        /// Content associated with a specific proposal
        ProposalContentHash get(fn proposal_content_hash):
            double_map hasher(blake2_256) OrgId<T>, hasher(blake2_256) ProposalIndex => Option<T::Hash>;

        /// This storage map encodes the default threshold for share types, proposal types
        /// - this is a helper for building the default stored below
        DefaultThresholdForShareIdProposalType get(fn default_threshold_for_share_id_proposal_type):
            double_map hasher(blake2_256) OrgSharePrefixKey<OrgId<T>, ShareId<T>>, hasher(blake2_256) ProposalType
            => Option<ThresholdConfig<Permill>>;

        /// This is the default approval order for shares for an organization, based on proposal types
        /// - this should not be used by itself for scheduling vote sequences, but allows us to easily build the other defaults
        ProposalDefaultShareApprovalOrderForOrganization get(fn proposal_default_share_approval_order_for_organization):
            double_map hasher(blake2_256) OrgId<T>, hasher(blake2_256) ProposalType => Option<Vec<ShareId<T>>>;

        /// This default may use the above default to set the default vote schedule for an organization
        ProposalDefaultVoteSchedule get(fn proposal_default_vote_schedule_for_organization):
            double_map hasher(blake2_256) OrgId<T>, hasher(blake2_256) ProposalType => Option<Vec<ScheduledVote<ShareId<T>, Permill>>>;

        /// TODO: replace with VoteSchedule for codomain
        LiveVoteSequences get(fn live_vote_sequences):
            double_map hasher(blake2_256) OrgId<T>, hasher(blake2_256) ProposalIndex => Option<VoteSchedule<ShareId<T>, VoteId<T>, Permill>>;

        /// Track the state of a proposal for a given organization
        /// TODO: replace with OutcomeContext
        ProposalState get(fn proposal_state):
            double_map hasher(blake2_256) OrgId<T>, hasher(blake2_256) ProposalIndex => Option<ProposalStage>;
    }
    add_extra_genesis {
        config(omnipotent_key): T::AccountId;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        const PollingFrequency: T::BlockNumber = T::PollingFrequency::get();

        fn on_finalize(n: T::BlockNumber) {
            if (n % T::PollingFrequency::get()).is_zero() {
                <AllActiveProposalKeys<T>>::get().iter().for_each(|(organization, proposal_index)| {
                    if let Ok(outcome) = Self::poll_active_proposal(*organization, *proposal_index) {
                        match outcome {
                            SimplePollingOutcome::MovedToNextVote(last_vote, new_vote) => {
                                Self::deposit_event(RawEvent::VoteScheduleProgress(*organization, *proposal_index, last_vote, new_vote));
                            },
                            // we could emit an event with the current vote_id here but that would be dumb IMO
                            SimplePollingOutcome::StayedOnCurrentVote(_) => {/*do nothing*/},
                            SimplePollingOutcome::Approved => {
                                // set the ProposalStage to Approved
                                // - note that we repeat this every time we iterate over the item after it's approved
                                // so we need to stay cognizant of this wasted computation and poorly resolved stale state
                                let new_proposal_stage = ProposalStage::Approved;
                                <ProposalState<T>>::insert(organization, proposal_index, new_proposal_stage);
                                Self::deposit_event(RawEvent::ProposalPassed(*organization, *proposal_index));
                            },
                        }
                    } // no error handling for error for now
                });
            }
        }

        fn register_organization(
            origin,
            initial_supervisor: Option<T::AccountId>,
            proposed_id: OrgId<T>,
            admin_share_id: ShareId<T>,
            genesis: Vec<(T::AccountId, SharesOf<T>)>,
            constitution: T::Hash,
        ) -> DispatchResult {
            let summoner = ensure_signed(origin)?;
            // only the sudo key can register new organizations
            ensure!(Self::is_sudo_key(&summoner), Error::<T>::OnlySudoKeyCanSummonForNow);
            let new_org_id = Self::generate_unique_id(proposed_id);
            let admin_id = <<T as Trait>::ShareData as ShareRegistration<
                    <T as frame_system::Trait>::AccountId>
                >::register(new_org_id, admin_share_id, genesis.into())?;
            // set the supervisor
            let d_supervisor = if let Some(supervisor) = initial_supervisor {
                // set the supervisor to the
                supervisor
            } else {
                // if None, then the supervisor is default the summoner (current sudo)
                summoner.clone()
            };
            <OrganizationSupervisor<T>>::insert(new_org_id, d_supervisor);
            // build new organization
            let new_organization = Organization::new(admin_id);
            // initialize organization storage items
            <OrganizationState<T>>::insert(new_org_id, new_organization);
            <ValueConstitution<T>>::insert(new_org_id, constitution);
            // TODO: map needs share_id context
            <OrganizationSupervisor<T>>::insert(new_org_id, summoner.clone());
            // Add one more organization to count (TODO: add subtract when organization is disbanded in that logic)
            let new_organization_count: u32 = OrganizationCount::get() + 1u32;
            OrganizationCount::put(new_organization_count);
            Self::deposit_event(RawEvent::NewOrganizationRegistered(summoner, new_org_id, admin_id));
            Ok(())
        }

        /// Update the value constitution from the organization's constitution's controller account
        fn update_value_constitution(
            origin,
            organization: OrgId<T>,
            new_constitution: T::Hash
        ) -> DispatchResult {
            // ensure there is an existing value constitution
            ensure!(!Self::id_is_available(organization), Error::<T>::NoExistingValueConstitution);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeConstitution);
            <ValueConstitution<T>>::insert(organization, new_constitution);
            Self::deposit_event(RawEvent::ConstitutionUpdated(author, organization, new_constitution));
            Ok(())
        }

        fn register_shares_in_organization(
            origin,
            organization: OrgId<T>,
            share_id: ShareId<T>,
            genesis: Vec<(T::AccountId, SharesOf<T>)>
        ) -> DispatchResult {
            // ensure that there is an existing organization (invariants elsewhere must ensure that this check syncs with all storage item changes)
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeConstitution);
            let new_share_id = <<T as Trait>::ShareData as ShareRegistration<
                <T as frame_system::Trait>::AccountId
            >>::register(organization, share_id, genesis.into())?;
            let old_organization = <OrganizationState<T>>::get(organization).ok_or(Error::<T>::NoRegisteredOrganizationWithIDProvided)?;
            let new_organization = old_organization.add_new_share_group(new_share_id);
            <OrganizationState<T>>::insert(organization, new_organization);
            Self::deposit_event(RawEvent::NewShareTypeRegisteredForOrganization(author, organization, new_share_id));
            Ok(())
        }

        fn create_single_vote_for_existing_share_group_in_organization(
            origin,
            organization: OrgId<T>,
            share_id: ShareId<T>,
            proposal_type: ProposalType,
            custom_threshold: Option<ThresholdConfig<Permill>>,
        ) -> DispatchResult {
            // ensure that there is an existing organization (invariants elsewhere must ensure that this check syncs with all storage item changes)
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeConstitution);
            let threshold = if let Some(thres_hold) = custom_threshold {
                thres_hold
            } else {
                let prefix_key = OrgSharePrefixKey::new(organization, share_id);
                <DefaultThresholdForShareIdProposalType<T>>::get(
                    prefix_key,
                    proposal_type,
                ).ok_or(Error::<T>::DefaultThresholdForShareIdNotSet)?
            };
            let new_vote_id = <<T as Trait>::BinaryVoteMachine as OpenVote<
                OrgId<T>,
                ShareId<T>,
                <T as frame_system::Trait>::AccountId,
                Permill,
            >>::open_vote(
                organization, share_id, None, threshold.into()
            )?;
            let now = system::Module::<T>::block_number();
            Self::deposit_event(RawEvent::SingleVoteCreatedForShareGroup(author, organization, share_id, new_vote_id, now));
            Ok(())
        }

        fn set_organization_share_id_proposal_type_default_threshold(
            origin,
            organization: OrgId<T>,
            share_id: ShareId<T>,
            proposal_type: ProposalType,
            passage_threshold_pct: Permill,
            turnout_threshold_pct: Permill,
        ) -> DispatchResult {
            // ensure that the organization has been registered
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let setter = ensure_signed(origin)?;
            let authentication: bool =
                Self::is_organization_supervisor(organization, &setter) || Self::is_sudo_key(&setter);
            ensure!(authentication, Error::<T>::NotAuthorizedToSetOrgShareIdProposalTypeThreshold);
            let threshold = ThresholdConfig::new(passage_threshold_pct, turnout_threshold_pct);
            Self::set_share_id_proposal_type_to_threshold(
                organization,
                share_id,
                proposal_type,
                threshold,
            )?;
            Self::deposit_event(
                RawEvent::NewOrgShareProposalThresholdSet(
                    setter,
                    organization,
                    share_id, proposal_type,
                    passage_threshold_pct,
                    turnout_threshold_pct
                ));
            Ok(())
        }

        // minimal vote-based configuration for the proposal_type
        fn set_most_basic_vote_requirements(
            origin,
            organization: OrgId<T>,
            proposal_type: ProposalType,
            ordered_share_ids: Vec<ShareId<T>>,
        ) -> DispatchResult {
            // ensure that there is an existing organization (invariants elsewhere must ensure that this check syncs with all storage item changes)
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToSetVoteRequirements);
            Self::set_default_share_approval_order_for_proposal_type(organization, proposal_type, ordered_share_ids)?;
            Self::deposit_event(RawEvent::MostBasicVoteRequirementSet(author, organization, proposal_type));
            Ok(())
        }

        fn make_proposal(
            origin,
            organization: OrgId<T>,
            proposal_type: ProposalType,
            content_reference: Option<T::Hash>,
        ) -> DispatchResult {
            // ensure that there is an existing organization (invariants elsewhere must ensure that this check syncs with all storage item changes)
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeConstitution);
            // new proposal index is the current proposal count plus one
            let proposal_index = <ProposalCount<T>>::get(organization) + 1;
            let first_scheduled_vote = Self::schedule_default_vote_schedule_for_proposal_type(organization, proposal_index, proposal_type, None)?;
            // insert content reference if it is input, it doesn't need to exist
            if let Some(content_hash) = content_reference {
                <ProposalContentHash<T>>::insert(organization, proposal_index, content_hash);
            }
            // set the proposal stage
            <ProposalState<T>>::insert(organization, proposal_index, ProposalStage::Voting);
            // add to AllActiveProposalKeys
            <AllActiveProposalKeys<T>>::mutate(|vecc| vecc.push((organization, proposal_index)));
            // update organization to include this proposal index
            let old_organization =
                <OrganizationState<T>>::get(
                    organization
                ).ok_or(Error::<T>::NoRegisteredOrganizationWithIDProvided)?;
            let new_organization = old_organization.add_proposal_index(proposal_index);
            <OrganizationState<T>>::insert(organization, new_organization);
            // this is the new organization proposal count for the organization
            <ProposalCount<T>>::insert(organization, proposal_index);
            let now = system::Module::<T>::block_number();
            Self::deposit_event(RawEvent::ProposalDispatchedToVote(author, organization, proposal_index, first_scheduled_vote, now));
            Ok(())
        }
    }
}

impl<T: Trait> IDIsAvailable<OrgId<T>> for Module<T> {
    fn id_is_available(id: OrgId<T>) -> bool {
        None == <ValueConstitution<T>>::get(id)
    }
}

impl<T: Trait> GenerateUniqueID<OrgId<T>> for Module<T> {
    fn generate_unique_id(proposed_id: OrgId<T>) -> OrgId<T> {
        if !Self::id_is_available(proposed_id) {
            let mut id_counter = <OrganizationIdentityNonce<T>>::get();
            while <ValueConstitution<T>>::get(id_counter).is_some() {
                // TODO: add overflow check here
                id_counter += 1.into();
            }
            <OrganizationIdentityNonce<T>>::put(id_counter + 1.into());
            id_counter
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> SudoKeyManagement<T::AccountId> for Module<T> {
    fn is_sudo_key(who: &T::AccountId) -> bool {
        if let Some(okey) = <SudoKey<T>>::get() {
            return who == &okey;
        }
        false
    }
    // only the sudo key can swap the sudo key (todo experiment: key recovery from some number of supervisors)
    fn swap_sudo_key(
        old_key: T::AccountId,
        new_key: T::AccountId,
    ) -> Result<T::AccountId, DispatchError> {
        if let Some(okey) = <SudoKey<T>>::get() {
            if old_key == okey {
                <SudoKey<T>>::put(new_key.clone());
                return Ok(new_key);
            }
            return Err(Error::<T>::UnAuthorizedSwapSudoRequest.into());
        }
        Err(Error::<T>::NoExistingSudoKeySoChainBricked.into())
    }
}

impl<T: Trait> SupervisorKeyManagement<OrgId<T>, T::AccountId> for Module<T> {
    fn is_organization_supervisor(organization: OrgId<T>, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::organization_supervisor(organization) {
            return who == &supervisor;
        }
        false
    }
    // sudo key and the current supervisor have the power to change the supervisor
    fn swap_supervisor(
        organization: OrgId<T>,
        old_key: T::AccountId,
        new_key: T::AccountId,
    ) -> Result<T::AccountId, DispatchError> {
        let authentication: bool =
            Self::is_organization_supervisor(organization, &old_key) || Self::is_sudo_key(&old_key);
        if authentication {
            <OrganizationSupervisor<T>>::insert(organization, new_key.clone());
            return Ok(new_key);
        }
        Err(Error::<T>::UnAuthorizedRequestToSwapSupervisor.into())
    }
}

impl<T: Trait> SetDefaultShareApprovalOrder<OrgId<T>, ShareId<T>> for Module<T> {
    type ProposalType = ProposalType;

    fn set_default_share_approval_order_for_proposal_type(
        organization: OrgId<T>,
        proposal_type: Self::ProposalType,
        share_approval_order: Vec<ShareId<T>>,
    ) -> DispatchResult {
        // TODO: check author permissions
        <ProposalDefaultShareApprovalOrderForOrganization<T>>::insert(
            organization,
            proposal_type,
            share_approval_order,
        );
        Ok(())
    }
}

impl<T: Trait> SetDefaultShareIdThreshold<OrgId<T>, ShareId<T>, Permill> for Module<T> {
    /// Warning: Must Be Permissioned, Config
    fn set_share_id_proposal_type_to_threshold(
        organization: OrgId<T>,
        share_id: ShareId<T>,
        proposal_type: Self::ProposalType,
        threshold: ThresholdConfig<Permill>,
    ) -> DispatchResult {
        let prefix_key = OrgSharePrefixKey::new(organization, share_id);
        <DefaultThresholdForShareIdProposalType<T>>::insert(prefix_key, proposal_type, threshold);
        Ok(())
    }
}

impl<T: Trait> VoteScheduleBuilder<OrgId<T>, ShareId<T>, Permill> for Module<T> {
    type ScheduledVote = ScheduledVote<ShareId<T>, Permill>;

    // setter
    fn scheduled_vote_from_share_id_proposal_type(
        organization: OrgId<T>,
        share_id: ShareId<T>,
        proposal_type: Self::ProposalType,
        // if None, use default set further above
        custom_threshold: Option<ThresholdConfig<Permill>>,
    ) -> Result<Self::ScheduledVote, DispatchError> {
        // use the custom threshold if Some
        let threshold = if let Some(cthreshold) = custom_threshold {
            cthreshold
        } else {
            // else use the default threshold but requires this to be set already
            let prefix_key = OrgSharePrefixKey::new(organization, share_id);
            <DefaultThresholdForShareIdProposalType<T>>::get(prefix_key, proposal_type)
                .ok_or(Error::<T>::DefaultThresholdForShareIdNotSet)?
        };
        Ok(ScheduledVote::new(0u32, share_id, threshold))
    }

    // converter for another method in a separate trait
    fn set_default_vote_schedule_for_proposal_type(
        organization: OrgId<T>,
        proposal_type: Self::ProposalType,
        // if None, use the default share approval order
        raw_vote_schedule: Option<Vec<Self::ScheduledVote>>,
    ) -> DispatchResult {
        if let Some(default_vote_schedule) = raw_vote_schedule {
            <ProposalDefaultVoteSchedule<T>>::insert(
                organization,
                proposal_type,
                default_vote_schedule,
            );
        } else {
            // use default share order for proposal type to derive default vote schedule
            let share_ids = <ProposalDefaultShareApprovalOrderForOrganization<T>>::get(
                organization,
                proposal_type,
            )
            .ok_or(Error::<T>::DevelopmentPlaceHolderError)?;
            let mut default_vote_schedule: Vec<Self::ScheduledVote> = Vec::new();
            for share_id in share_ids.iter() {
                // uses the default threshold set in the vote module through a trait
                let new_scheduled_vote = Self::scheduled_vote_from_share_id_proposal_type(
                    organization,
                    *share_id,
                    proposal_type,
                    // using default threshold for share_id_proposal_type
                    None,
                )?;
                default_vote_schedule.push(new_scheduled_vote);
            }
            <ProposalDefaultVoteSchedule<T>>::insert(
                organization,
                proposal_type,
                default_vote_schedule,
            );
        }
        Ok(())
    }
}

impl<T: Trait> VoteScheduler<OrgId<T>, ShareId<T>, VoteId<T>> for Module<T> {
    type VoteSchedule = VoteSchedule<ShareId<T>, VoteId<T>, Permill>;

    fn dispatch_vote_schedule_from_vec_of_share_id(
        organization: OrgId<T>,
        proposal_type: ProposalType,
        share_ids: Vec<ShareId<T>>,
    ) -> Result<Self::VoteSchedule, DispatchError> {
        let mut next_ordered_scheduled_votes: Vec<ScheduledVote<ShareId<T>, Permill>> = Vec::new();
        let mut votes_left_including_current = 0u32;
        let mut first_dispatched_vote_id: VoteId<T> = 0.into();
        let mut first_share_id: ShareId<T> = 0.into();
        for share in share_ids.iter() {
            // iterate through share_ids and get default threshold config
            // here use the default threshold config for the share_id
            // TODO: add `custom_thresholds` field and branch of logic
            if let Ok(new_vote_to_schedule) = Self::scheduled_vote_from_share_id_proposal_type(
                organization,
                *share,
                proposal_type,
                // using default share_id_proposal_type_to_threshold
                None,
            ) {
                // if first vote, then schedule it now and store the rest in storage
                if votes_left_including_current == 0 {
                    // open vote with default configuration
                    let prefix_key = OrgSharePrefixKey::new(organization, *share);
                    let threshold =
                        <DefaultThresholdForShareIdProposalType<T>>::get(prefix_key, proposal_type)
                            .ok_or(Error::<T>::DefaultThresholdForShareIdNotSet)?;
                    let new_vote_id = <<T as Trait>::BinaryVoteMachine as OpenVote<
                        OrgId<T>,
                        ShareId<T>,
                        <T as frame_system::Trait>::AccountId,
                        Permill,
                    >>::open_vote(
                        organization, *share, None, threshold.into()
                    )?;
                    first_dispatched_vote_id = new_vote_id;
                    first_share_id = *share;
                } else {
                    next_ordered_scheduled_votes.push(new_vote_to_schedule);
                }
                votes_left_including_current += 1u32;
            } else {
                //
                return Err(Error::<T>::NoVoteStarted.into());
            }
            // else, do nothing now -- illegitimate shares are ignored for now but we could propagate errors later to be more comprehensive
            // NOTICE: this will trigger an error while testing if the vector of `ShareId`s are not already correctly registered in the inherited modules
        }
        ensure!(
            first_dispatched_vote_id != 0.into() && first_share_id != 0.into(),
            Error::<T>::NoVoteStarted
        );
        Ok(VoteSchedule::new(
            first_share_id,
            first_dispatched_vote_id,
            next_ordered_scheduled_votes,
        ))
    }

    // should only be called in highly constrained settings
    fn move_to_next_scheduled_vote(
        organization: OrgId<T>,
        schedule: Self::VoteSchedule,
    ) -> Result<Option<Self::VoteSchedule>, DispatchError> {
        let mutable_schedule = schedule.get_schedule();
        let current_len = &mutable_schedule.len();
        let (current_share_id, current_vote_id) = if let Some(vote_left) =
            mutable_schedule.clone().pop()
        {
            let share_id = vote_left.get_share_id();
            let threshold = vote_left.get_threshold();
            // open vote with default configuration
            let new_vote_id =
                <<T as Trait>::BinaryVoteMachine as OpenVote<
                    OrgId<T>,
                    ShareId<T>,
                    <T as frame_system::Trait>::AccountId,
                    Permill,
                >>::open_vote(organization, share_id, None, threshold.into())?;
            (share_id, new_vote_id)
        } else {
            // the proposal has passed because there are no votes left on the schedule and the current vote passed
            // NOTE: if there are ever any error branches that are fallible, they cannot return Ok(None), instead DispatchError
            return Ok(None);
        };
        // there should be one less argument after the pop or I need to do it again because of if let Some semantics -- TODO: TEST THIS METHOD
        ensure!(
            &mutable_schedule.len() != current_len,
            Error::<T>::DevelopmentPlaceHolderError
        );
        Ok(Some(VoteSchedule::new(
            current_share_id,
            current_vote_id,
            mutable_schedule,
        )))
    }
}

impl<T: Trait> ScheduleVoteSequence<OrgId<T>, ShareId<T>, VoteId<T>, Permill> for Module<T> {
    type ProposalIndex = ProposalIndex;

    fn schedule_default_vote_schedule_for_proposal_type(
        organization: OrgId<T>,
        index: Self::ProposalIndex,
        proposal_type: Self::ProposalType,
        // if None, just use the default vote schedule
        custom_share_ids: Option<Vec<ShareId<T>>>,
    ) -> Result<VoteId<T>, DispatchError> {
        let share_ids = if let Some(custom_list) = custom_share_ids {
            custom_list
        } else {
            // get the default list of necessary approved shares
            if let Some(default_list_for_share_type) =
                <ProposalDefaultShareApprovalOrderForOrganization<T>>::get(
                    organization,
                    proposal_type,
                )
            {
                default_list_for_share_type
            } else {
                return Err(Error::<T>::NoDefaultSetAndNoShareVoteScheduleProvided.into());
            }
        };
        // schedule the sequence in this call
        let new_vote_schedule = Self::dispatch_vote_schedule_from_vec_of_share_id(
            organization,
            proposal_type,
            share_ids,
        )?;
        let first_vote_id = new_vote_schedule.get_current_vote_id();
        // set the scheduled the vote sequence in storage
        <LiveVoteSequences<T>>::insert(organization, index, new_vote_schedule);
        // TODO: emit an event here or in the callee's method
        Ok(first_vote_id)
    }
}

/// Checks the progress of a scheduled vote sequence and pushes the schedule along
/// - this should be called every `T::PollingFrequency::get()` number of blocks in `on_finalize`
impl<T: Trait> PollActiveProposal<OrgId<T>, ShareId<T>, VoteId<T>, Permill> for Module<T> {
    type PollingOutcome = SimplePollingOutcome<VoteId<T>>;

    // This method checks the outcome of the current vote and moves the schedule to the next one when the threshold is met
    // - returns the newest `VoteId` when the voting schedule is pushed to the next vote
    fn poll_active_proposal(
        organization: OrgId<T>,
        index: Self::ProposalIndex,
    ) -> Result<Self::PollingOutcome, DispatchError> {
        let active_schedule = <LiveVoteSequences<T>>::get(organization, index)
            .ok_or(Error::<T>::NoScheduledVoteSequence)?;
        // if there is no current vote, something is wrong TODO: think about this
        let share_id = active_schedule.get_current_share_id();
        let vote_id = active_schedule.get_current_vote_id();
        // check the outcome of the current vote
        let current_outcome = <<T as Trait>::BinaryVoteMachine as GetVoteOutcome<
            OrgId<T>,
            ShareId<T>,
        >>::get_vote_outcome(organization, share_id, vote_id)?;
        if current_outcome.approved() {
            let wrapped_new_schedule =
                Self::move_to_next_scheduled_vote(organization, active_schedule)?;
            if let Some(schedule) = wrapped_new_schedule {
                let next_vote_id = schedule.get_current_vote_id();
                <LiveVoteSequences<T>>::insert(organization, index, schedule);
                return Ok(Self::PollingOutcome::MovedToNextVote(vote_id, next_vote_id));
            } else {
                return Ok(Self::PollingOutcome::Approved);
            }
        }
        Ok(Self::PollingOutcome::StayedOnCurrentVote(vote_id))
    }
}
