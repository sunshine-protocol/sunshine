#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
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
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    DispatchError,
    DispatchResult,
    Permill, // ModuleId //add to Organization struct once substrate#5149 is addressed
};
use sp_std::{fmt::Debug, prelude::*};
use util::{
    organization::Organization,
    proposal::{ProposalIndex, ProposalStage, ProposalType, SimplePollingOutcome},
    traits::{
        Approved,
        GenerateUniqueID,
        GetVoteOutcome,
        IDIsAvailable,
        OpenVote,
        PollActiveProposal,
        ReservableProfile,
        ScheduleDefaultVoteSchedule,
        ScheduledVoteBuilder,
        SetDefaultShareApprovalOrder,
        SetDefaultVoteSchedule,
        ShareRegistration,
        SudoKeyManagement,
        SupervisorKeyManagement,
        VoteOnProposal, // ScheduleCustomVoteSequence
    },
    vote::{ScheduledVote, ThresholdConfig, VoteSchedule},
};

/// The share identifier type for binary votes (associated with the Trait's binary vote machine)
pub type BinaryShareId<T> = <<<T as Trait>::BinaryVoteMachine as OpenVote<
    <T as frame_system::Trait>::AccountId,
    Permill,
>>::ShareRegistrar as ReservableProfile<<T as frame_system::Trait>::AccountId>>::ShareId;

/// The share type for binary votes (associated with the Trait's binary vote machine)
pub type BinaryShares<T> = <<<T as Trait>::BinaryVoteMachine as OpenVote<
    <T as frame_system::Trait>::AccountId,
    Permill,
>>::ShareRegistrar as ReservableProfile<<T as frame_system::Trait>::AccountId>>::Shares;

/// The vote identifier type for binary votes
pub type BinaryVoteId<T> = <<T as Trait>::BinaryVoteMachine as GetVoteOutcome>::VoteId;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The organization identifier
    type OrgId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + From<u32>;

    /// The `vote-yesno` module instance
    type BinaryVoteMachine: GetVoteOutcome
        + OpenVote<Self::AccountId, Permill>
        + VoteOnProposal<Self::AccountId, Permill>
        + IDIsAvailable<BinaryVoteId<Self>>
        + GenerateUniqueID<BinaryVoteId<Self>>;

    /// The number of blocks between the polling of all active proposals for all organizations (see issue #84)
    type PollingFrequency: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        OrgId = <T as Trait>::OrgId,
        ShareId = BinaryShareId<T>,
        VoteId = BinaryVoteId<T>,
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
            map hasher(blake2_256) T::OrgId => Option<T::AccountId>;

        /// The identity generator nonce for T::OrgId
        OrganizationIdentityNonce get(fn organization_identity_nonce): T::OrgId;

        /// The number of organizations registered on-chain
        OrganizationCount get(fn organization_count): u32;

        /// The organization's state
        /// TODO: using the id as the key and also keeping it inside the `Organization` struct seems redundant
        OrganizationState get(fn organization_state):
            map hasher(blake2_256) T::OrgId => Option<Organization<T::OrgId, BinaryShareId<T>>>;

        ProposalContentHash get(fn proposal_content_hash):
            double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) ProposalIndex => Option<T::Hash>;

        /// This constitution should be used to guide voting
        /// - it is also the main anchor for any organization and therefore defines registration
        ValueConstitution get(fn value_constitution):
            map hasher(blake2_256) T::OrgId => Option<T::Hash>;

        /// The total number of proposals for an organization, also used as the nonce
        ProposalCount get(fn proposal_count): map hasher(blake2_256) T::OrgId => ProposalIndex;

        /// Every time a proposal is added, it should be added to this
        /// - this storage item is used in on_finalize to poll all active proposals (see `PollActiveProposal` trait)
        AllActiveProposalKeys get(fn all_active_proposal_keys): Vec<(T::OrgId, ProposalIndex)>;

        /// This storage map encodes the default threshold for share types, proposal types
        /// - this is a helper for building the default stored below
        pub DefaultThresholdForShareIdProposalType get(fn default_threshold_for_share_id_proposal_type):
            double_map hasher(blake2_256) (T::OrgId, BinaryShareId<T>), hasher(blake2_256) ProposalType
            => Option<ThresholdConfig<Permill>>;

        /// This is the default approval order for shares for an organization, based on proposal types
        /// - this should not be used by itself for scheduling vote sequences, but allows us to easily build the other defaults
        ProposalDefaultShareApprovalOrderForOrganization get(fn proposal_default_share_approval_order_for_organization):
            double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) ProposalType => Option<Vec<BinaryShareId<T>>>;

        /// This default may use the above default to set the default vote schedule for an organization
        ProposalDefaultVoteSchedule get(fn proposal_default_vote_schedule_for_organization):
            double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) ProposalType => Option<Vec<ScheduledVote<BinaryShareId<T>, Permill>>>;

        /// TODO: replace with VoteSchedule for codomain
        VoteSequences get(fn vote_sequences):
            double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) ProposalIndex => Option<VoteSchedule<BinaryVoteId<T>, BinaryShareId<T>, Permill>>;

        /// Track the state of a proposal for a given organization
        /// TODO: replace with OutcomeContext
        OrgProposalStage get(fn org_proposal_stage):
            double_map hasher(blake2_256) T::OrgId, hasher(blake2_256) ProposalIndex => Option<ProposalStage>;
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
                let _ = <AllActiveProposalKeys<T>>::get().iter().for_each(|(organization, proposal_index)| {
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
                                <OrgProposalStage<T>>::insert(organization, proposal_index, new_proposal_stage);
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
            proposed_id: T::OrgId,
            constitution: T::Hash,
            admin_share_id: BinaryShareId<T>,
            genesis: Vec<(T::AccountId, BinaryShares<T>)>,
        ) -> DispatchResult {
            let summoner = ensure_signed(origin)?;
            // only the sudo key can register new organizations
            ensure!(Self::is_sudo_key(&summoner), Error::<T>::OnlySudoKeyCanSummonForNow);
            let new_org_id = Self::generate_unique_id(proposed_id);
            let admin_id = <<<T as Trait>::BinaryVoteMachine as OpenVote<
                <T as frame_system::Trait>::AccountId,
                Permill
            >>::ShareRegistrar as ShareRegistration<
                <T as frame_system::Trait>::AccountId>
            >::register(admin_share_id, genesis.into())?;
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
            let new_organization = Organization::new(new_org_id, admin_id);
            // initialize organization storage items
            <OrganizationState<T>>::insert(new_org_id, new_organization);
            <ValueConstitution<T>>::insert(new_org_id, constitution);
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
            organization: T::OrgId,
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
            organization: T::OrgId,
            share_id: BinaryShareId<T>,
            genesis: Vec<(T::AccountId, BinaryShares<T>)>
        ) -> DispatchResult {
            // ensure that there is an existing organization (invariants elsewhere must ensure that this check syncs with all storage item changes)
            ensure!(!Self::id_is_available(organization), Error::<T>::NoRegisteredOrganizationWithIDProvided);
            let author = ensure_signed(origin)?;
            let authentication: bool = Self::is_organization_supervisor(organization, &author) || Self::is_sudo_key(&author);
            ensure!(authentication, Error::<T>::NotAuthorizedToChangeConstitution);
            let new_share_id = <<<T as Trait>::BinaryVoteMachine as OpenVote<
                <T as frame_system::Trait>::AccountId,
                Permill
            >>::ShareRegistrar as ShareRegistration<
                <T as frame_system::Trait>::AccountId>
            >::register(share_id, genesis.into())?;
            let old_organization = <OrganizationState<T>>::get(organization).ok_or(Error::<T>::NoRegisteredOrganizationWithIDProvided)?;
            let new_organization = old_organization.add_new_share_group(new_share_id);
            <OrganizationState<T>>::insert(organization, new_organization);
            Self::deposit_event(RawEvent::NewShareTypeRegisteredForOrganization(author, organization, new_share_id));
            Ok(())
        }

        fn set_organization_share_id_proposal_type_default_threshold(
            origin,
            organization: T::OrgId,
            share_id: BinaryShareId<T>,
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
            Self::set_share_id_proposal_type_to_threshold(
                organization, share_id,
                proposal_type,
                passage_threshold_pct,
                turnout_threshold_pct
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
            organization: T::OrgId,
            proposal_type: ProposalType,
            ordered_share_ids: Vec<BinaryShareId<T>>,
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
            organization: T::OrgId,
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
            <OrgProposalStage<T>>::insert(organization, proposal_index, ProposalStage::Voting);
            // add to AllActiveProposalKeys
            <AllActiveProposalKeys<T>>::mutate(|vecc| vecc.push((organization, proposal_index)));
            // this is the new organization proposal count for the organization
            <ProposalCount<T>>::insert(organization, proposal_index.clone());
            let now = system::Module::<T>::block_number();
            Self::deposit_event(RawEvent::ProposalDispatchedToVote(author, organization, proposal_index, first_scheduled_vote, now));
            Ok(())
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
    // only the sudo key can swap the sudo key (experiment: key recovery from some number of supervisors)
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

impl<T: Trait> SupervisorKeyManagement<T::AccountId, T::OrgId> for Module<T> {
    fn is_organization_supervisor(organization: T::OrgId, who: &T::AccountId) -> bool {
        if let Some(supervisor) = Self::organization_supervisor(organization) {
            return who == &supervisor;
        }
        false
    }
    // sudo key and the current supervisor have the power to change the supervisor
    fn swap_supervisor(
        organization: T::OrgId,
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

impl<T: Trait> IDIsAvailable<T::OrgId> for Module<T> {
    fn id_is_available(id: T::OrgId) -> bool {
        None == <ValueConstitution<T>>::get(id)
    }
}

impl<T: Trait> GenerateUniqueID<T::OrgId> for Module<T> {
    fn generate_unique_id(proposed_id: T::OrgId) -> T::OrgId {
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

impl<T: Trait> SetDefaultShareApprovalOrder<BinaryShareId<T>, T::OrgId> for Module<T> {
    type ProposalType = ProposalType;

    fn set_default_share_approval_order_for_proposal_type(
        organization: T::OrgId,
        proposal_type: Self::ProposalType,
        share_approval_order: Vec<BinaryShareId<T>>,
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

impl<T: Trait> ScheduledVoteBuilder<BinaryShareId<T>, T::OrgId, Permill> for Module<T> {
    type ScheduledVote = ScheduledVote<BinaryShareId<T>, Permill>;

    // setter
    fn set_share_id_proposal_type_to_threshold(
        organization: T::OrgId,
        share_id: BinaryShareId<T>,
        proposal_type: Self::ProposalType,
        passage_threshold_pct: Permill,
        turnout_threshold_pct: Permill,
    ) -> DispatchResult {
        let threshold_config = ThresholdConfig::new(passage_threshold_pct, turnout_threshold_pct);
        <DefaultThresholdForShareIdProposalType<T>>::insert(
            (organization, share_id),
            proposal_type,
            threshold_config,
        );
        Ok(())
    }

    // converter for another method in a separate trait
    fn scheduled_vote_from_share_id_proposal_type(
        organization: T::OrgId,
        share_id: BinaryShareId<T>,
        proposal_type: Self::ProposalType,
    ) -> Result<Self::ScheduledVote, DispatchError> {
        // this threshold is set for the share type in the vote module (because it is a default in this context)
        let default_threshold = <DefaultThresholdForShareIdProposalType<T>>::get(
            (organization, share_id),
            proposal_type,
        )
        .ok_or(Error::<T>::DefaultThresholdForShareIdNotSet)?;
        Ok(ScheduledVote {
            priority: 0u32,
            proposal_type,
            share_type: share_id,
            threshold: default_threshold.into(),
        })
    }
}

impl<T: Trait> SetDefaultVoteSchedule<BinaryShareId<T>, T::OrgId, Permill> for Module<T> {
    fn set_default_vote_schedule_for_proposal_type(
        organization: T::OrgId,
        proposal_type: Self::ProposalType,
        raw_vote_schedule: Option<Vec<Self::ScheduledVote>>,
    ) -> DispatchResult {
        // TODO: check author permissions
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

impl<T: Trait> ScheduleDefaultVoteSchedule<BinaryShareId<T>, BinaryVoteId<T>, T::OrgId, Permill>
    for Module<T>
{
    type ProposalIndex = ProposalIndex;

    fn schedule_default_vote_schedule_for_proposal_type(
        organization: T::OrgId,
        index: Self::ProposalIndex,
        proposal_type: Self::ProposalType,
        custom_share_ids: Option<Vec<BinaryShareId<T>>>,
    ) -> Result<BinaryVoteId<T>, DispatchError> {
        // iterate through share_ids and get default threshold config
        let mut next_ordered_scheduled_votes: Vec<ScheduledVote<BinaryShareId<T>, Permill>> =
            Vec::new();
        let mut votes_left_including_current = 0u32;
        let mut starting_vote: Option<BinaryVoteId<T>> = None;
        // get the ProposalDefault
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
        for share in share_ids.clone().iter() {
            // here use the default threshold config for the share_id
            // TODO: add `custom_thresholds` field and branch of logic
            if let Ok(new_vote_to_schedule) = Self::scheduled_vote_from_share_id_proposal_type(
                organization,
                *share,
                proposal_type,
            ) {
                // if first vote, then schedule it now and store the rest in storage
                if votes_left_including_current == 0 {
                    // a very dumb vote_id generation algorithm
                    let weak_attempted_vote_id: BinaryVoteId<T> = 69u32.into();
                    // open vote with default configuration
                    let threshold = <DefaultThresholdForShareIdProposalType<T>>::get(
                        (organization, *share),
                        proposal_type,
                    )
                    .ok_or(Error::<T>::DefaultThresholdForShareIdNotSet)?;
                    let new_vote_id = <<T as Trait>::BinaryVoteMachine as OpenVote<
                        <T as frame_system::Trait>::AccountId,
                        Permill,
                    >>::open_vote(
                        weak_attempted_vote_id,
                        *share,
                        proposal_type.into(),
                        threshold.passage_threshold_pct,
                        threshold.turnout_threshold_pct,
                    )?;
                    starting_vote = Some(new_vote_id);
                } else {
                    next_ordered_scheduled_votes.push(new_vote_to_schedule);
                }
                votes_left_including_current += 1u32;
            }
            // else, do nothing now -- illegitimate shares are ignored for now but we could propagate errors later to be more comprehensive
            // NOTICE: this will trigger an error while testing if the vector of `ShareId`s are not already correctly registered in the inherited modules
        }
        // return error here if error exists (QUESTION: does unwrap propagate the error as I expect it does)
        let returned_first_vote = starting_vote.ok_or(Error::<T>::NoVoteStarted)?;
        // TODO: ensure that at least one more vote is in schedule
        let new_vote_schedule = VoteSchedule {
            votes_left_including_current,
            current_vote: Some(returned_first_vote),
            schedule: next_ordered_scheduled_votes,
        };
        // schedule the vote sequence
        <VoteSequences<T>>::insert(organization, index, new_vote_schedule);
        // TODO: emit an event here or in the callee's method
        Ok(returned_first_vote)
    }
}

/// Checks the progress of a scheduled vote sequence and pushes the schedule along
/// - this should be called every `T::PollingFrequency::get()` number of blocks in `on_finalize`
impl<T: Trait> PollActiveProposal<BinaryShareId<T>, BinaryVoteId<T>, T::OrgId, Permill>
    for Module<T>
{
    type PollingOutcome = SimplePollingOutcome<BinaryVoteId<T>>;

    // This method checks the outcome of the current vote and moves the schedule to the next one when the threshold is met
    // - returns the newest `VoteId` when the voting schedule is pushed to the next vote
    fn poll_active_proposal(
        organization: T::OrgId,
        index: Self::ProposalIndex,
    ) -> Result<Self::PollingOutcome, DispatchError> {
        let mut active_schedule = <VoteSequences<T>>::get(organization, index)
            .ok_or(Error::<T>::NoScheduledVoteSequence)?;
        // if there is no current vote, something is wrong TODO: think about this
        let old_current_vote_id = active_schedule
            .current_vote
            .ok_or(Error::<T>::DevelopmentPlaceHolderError)?;
        let mut votes_left_including_current = active_schedule.votes_left_including_current;
        // check the outcome of the current vote
        let current_outcome =
            <<T as Trait>::BinaryVoteMachine as GetVoteOutcome>::get_vote_outcome(
                old_current_vote_id,
            )?;
        let mut cheap_id_generation_nonce: BinaryVoteId<T> = 0u32.into();
        if current_outcome.approved() {
            // TODO: check if I should pop here or rev and pop (what direction are we inserting and removing elements)
            let next_vote_id = if let Some(vote_left) = active_schedule.schedule.pop() {
                // a very dumb vote_id generation algorithm
                cheap_id_generation_nonce += 1.into();
                // open vote with default configuration
                let get_new_vote_id = <<T as Trait>::BinaryVoteMachine as OpenVote<
                    <T as frame_system::Trait>::AccountId,
                    Permill,
                >>::open_vote(
                    cheap_id_generation_nonce,
                    vote_left.share_type,
                    vote_left.proposal_type.into(),
                    vote_left.threshold.passage_threshold_pct,
                    vote_left.threshold.turnout_threshold_pct,
                )?;
                Some(get_new_vote_id)
            } else {
                // the proposal has passed because there are no votes left on the schedule
                // and the current vote passed
                return Ok(Self::PollingOutcome::Approved);
            };
            votes_left_including_current -= 1;
            // TODO: replace with `new` method
            let new_vote_schedule = VoteSchedule {
                votes_left_including_current,
                current_vote: next_vote_id,
                // this should have been mutated by `pop`
                schedule: active_schedule.schedule,
            };
            <VoteSequences<T>>::insert(organization, index, new_vote_schedule);
            let new_current_vote_id =
                next_vote_id.expect("returned proposal approval if none in else branch above; qed");
            return Ok(Self::PollingOutcome::MovedToNextVote(
                old_current_vote_id,
                new_current_vote_id,
            ));
        }
        Ok(Self::PollingOutcome::StayedOnCurrentVote(
            old_current_vote_id,
        ))
    }
}
