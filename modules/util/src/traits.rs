use codec::FullCodec;
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize},
    DispatchError, DispatchResult,
};
use sp_std::{fmt::Debug, prelude::*};

// === Unique ID Logic, Useful for All Modules ==

// For the module to implement for its id type
pub trait IDIsAvailable<Id> {
    fn id_is_available(id: Id) -> bool;
}

pub trait GenerateUniqueID<Id>: IDIsAvailable<Id> {
    // this should be infallible, it returns the generated unique id which may or may not be equal to the original value
    fn generate_unique_id(proposed_id: Id) -> Id;
}

// ---------- Share Logic ----------

pub trait VerifyShape {
    // required bound on GenesisAllocation
    fn verify_shape(&self) -> bool;
}

pub trait GroupMembership<AccountId> {
    type GroupId;

    fn is_member_of_group(group_id: Self::GroupId, who: &AccountId) -> bool;
}

/// For the module, to encode share registration behavior
pub trait ShareRegistration<AccountId>: GroupMembership<AccountId> {
    type OrgId: AtLeast32Bit
        + FullCodec
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Default
        + From<u32>;
    type ShareId: AtLeast32Bit
        + FullCodec
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Default
        + From<u32>;
    type Shares: AtLeast32Bit + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    type GenesisAllocation: From<Vec<(AccountId, Self::Shares)>> + VerifyShape;

    fn register(
        organization: Self::OrgId,
        proposed_id: Self::ShareId,
        genesis_allocation: Self::GenesisAllocation,
    ) -> Result<Self::ShareId, DispatchError>;
}

/// Traits for the ReservationContext object on ReservableProfile
pub trait GetMagnitude<Shares> {
    fn get_magnitude(self) -> Shares;
}
use frame_support::Parameter;
impl<Shares: Parameter> GetMagnitude<Shares> for (u32, Shares) {
    fn get_magnitude(self) -> Shares {
        self.1
    }
}
impl<Shares: Parameter> GetMagnitude<Shares> for Shares {
    fn get_magnitude(self) -> Shares {
        self
    }
}

/// For the module, to abstract the share reservation behavior
pub trait ReservableProfile<AccountId>: ShareRegistration<AccountId> {
    type ReservationContext: GetMagnitude<Self::Shares>;
    /// Reserves amount iff certain conditions are met wrt existing profile and how it would change
    fn reserve(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        who: &AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError>;
    /// Unreserves amount iff certain conditions are met wrt existing profile and how it would change
    fn unreserve(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        who: &AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError>;
}

pub trait LockableProfile<AccountId>: ShareRegistration<AccountId> {
    fn lock_profile(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        who: &AccountId,
    ) -> DispatchResult;
    fn unlock_profile(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        who: &AccountId,
    ) -> DispatchResult;
}

pub trait GetProfile<AccountId>: ShareRegistration<AccountId> {
    fn get_share_profile(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        who: &AccountId,
    ) -> Result<Self::Shares, DispatchError>;
}

/// For the module, to separate the issuance logic for shares
pub trait ShareBank<AccountId>: ShareRegistration<AccountId> {
    fn outstanding_shares(organization: Self::OrgId, id: Self::ShareId) -> Self::Shares;
    // return membership group associated with a share type
    fn shareholder_membership(
        organization: Self::OrgId,
        id: Self::ShareId,
    ) -> Result<Vec<AccountId>, DispatchError>;
    fn issue(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        new_owner: &AccountId,
        amount: Self::Shares,
    ) -> DispatchResult;
    fn burn(
        organization: Self::OrgId,
        share_id: Self::ShareId,
        old_owner: &AccountId,
        amount: Self::Shares,
    ) -> DispatchResult;
}

// ====== Vote Logic ======

pub trait GetVoteOutcome<OrgId, ShareId> {
    type VoteId: AtLeast32Bit
        + FullCodec
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Default
        + From<u32>;
    type Outcome: Approved;

    fn get_vote_outcome(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
    ) -> Result<Self::Outcome, DispatchError>;
}

// For ThresholdConfig to derive the threshold requirement from turnout
pub trait DeriveThresholdRequirement<Signal> {
    fn derive_support_requirement(&self, turnout: Signal) -> Signal;
    fn derive_turnout_requirement(&self, turnout: Signal) -> Signal;
}

use crate::vote::ThresholdConfig;

pub trait VoteThresholdBuilder<FineArithmetic> {
    type Signal: AtLeast32Bit + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    type ThresholdConfig: DeriveThresholdRequirement<Self::Signal>;
    type VoteThreshold;

    fn build_vote_threshold(
        threshold_config: Self::ThresholdConfig,
        possible_turnout: Self::Signal,
    ) -> Self::VoteThreshold;
}

pub trait OpenVote<OrgId, ShareId, AccountId, FineArithmetic>:
    GetVoteOutcome<OrgId, ShareId> + VoteThresholdBuilder<FineArithmetic>
{
    fn open_vote(
        organization: OrgId,
        share_id: ShareId,
        // uuid generation should default happen when this is called
        vote_id: Option<Self::VoteId>,
        threshold_config: ThresholdConfig<FineArithmetic>,
    ) -> Result<Self::VoteId, DispatchError>;
}

// access point for signal
pub trait MintableSignal<OrgId, ShareId, AccountId, FineArithmetic>:
    OpenVote<OrgId, ShareId, AccountId, FineArithmetic>
{
    fn mint_signal_based_on_existing_share_value(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
        who: &AccountId,
    ) -> Result<Self::Signal, DispatchError>;

    /// WARNING: CALL MUST BE PERMISSIONED
    fn custom_mint_signal(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
        who: &AccountId,
        amount: Self::Signal,
    ) -> Result<Self::Signal, DispatchError>;

    /// Mints signal for all ShareIds
    /// - calls mint_signal_based_on_existing_share_value for every member
    /// - TODO: add custom_batch_mint_signal
    fn batch_mint_signal(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
    ) -> Result<Self::Signal, DispatchError>;
}

pub trait BurnableSignal<OrgId, ShareId, AccountId, FineArithmetic>:
    MintableSignal<OrgId, ShareId, AccountId, FineArithmetic>
{
    fn burn_signal(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
        who: &AccountId,
        amount: Option<Self::Signal>,
    ) -> DispatchResult;
}

/// For VoteState, to verify passage
pub trait Approved {
    fn approved(&self) -> bool;
}

pub trait VoteVector<Magnitude, Direction> {
    fn magnitude(&self) -> Magnitude;
    fn direction(&self) -> Direction;
}

use crate::vote::YesNoVote;
/// For module to apply the vote in the context of the existing module instance
pub trait ApplyVote {
    type Magnitude;
    type Direction;
    // TODO: instead of `From<YesNoVote<Self::Magnitude>>`, we want a trait
    // that takes Magnitude, Direction and creates a new VoteVector
    type Vote: From<YesNoVote<Self::Magnitude>> + VoteVector<Self::Magnitude, Self::Direction>;
    type State: Approved;

    fn apply_vote(
        state: Self::State,
        new_vote: Self::Vote,
        old_vote: Option<Self::Vote>,
    ) -> Result<Self::State, DispatchError>;
}

/// For the module to check the status of the vote in the context of the existing module instance
pub trait CheckVoteStatus<OrgId, ShareId>: ApplyVote + GetVoteOutcome<OrgId, ShareId> {
    fn check_vote_outcome(state: Self::State) -> Result<Self::Outcome, DispatchError>;
    fn check_vote_expired(state: Self::State) -> bool;
}

/// For module to update vote state
pub trait VoteOnProposal<OrgId, ShareId, AccountId, FineArithmetic>:
    OpenVote<OrgId, ShareId, AccountId, FineArithmetic> + CheckVoteStatus<OrgId, ShareId>
{
    fn vote_on_proposal(
        organization: OrgId,
        share_id: ShareId,
        vote_id: Self::VoteId,
        voter: &AccountId,
        direction: Self::Direction,
        magnitude: Option<Self::Magnitude>,
    ) -> DispatchResult;
}

// ====== Vote Dispatch Logic (in Bank) ======

// for the VoteSchedule struct (most other traits are for the module itself)
pub trait GetCurrentVoteIdentifiers<ShareId, VoteId> {
    fn get_current_share_id(&self) -> ShareId;
    fn get_current_vote_id(&self) -> VoteId;
}

pub trait SetDefaultShareApprovalOrder<OrgId, ShareId> {
    type ProposalType;

    fn set_default_share_approval_order_for_proposal_type(
        organization: OrgId,
        proposal_type: Self::ProposalType,
        share_approval_order: Vec<ShareId>,
    ) -> DispatchResult;
}

pub trait SetDefaultShareIdThreshold<OrgId, ShareId, FineArithmetic>:
    SetDefaultShareApprovalOrder<OrgId, ShareId>
{
    fn set_share_id_proposal_type_to_threshold(
        organization: OrgId,
        share_id: ShareId,
        proposal_type: Self::ProposalType,
        threshold: ThresholdConfig<FineArithmetic>,
    ) -> DispatchResult;
}

pub trait VoteScheduleBuilder<OrgId, ShareId, FineArithmetic>:
    SetDefaultShareIdThreshold<OrgId, ShareId, FineArithmetic>
{
    type ScheduledVote;

    // uses the default threshold set above to automatically set threshold
    fn scheduled_vote_from_share_id_proposal_type(
        organization: OrgId,
        share_id: ShareId,
        proposal_type: Self::ProposalType,
        // if None, use default set further above
        custom_threshold: Option<ThresholdConfig<FineArithmetic>>,
    ) -> Result<Self::ScheduledVote, DispatchError>;

    fn set_default_vote_schedule_for_proposal_type(
        organization: OrgId,
        proposal_type: Self::ProposalType,
        // if None, use the default share approval order
        raw_vote_schedule: Option<Vec<Self::ScheduledVote>>,
    ) -> DispatchResult;
}

/// NOTE: I want to do something similar with `ThresholdConfig` to make the object definition more generic
/// but without fucking up all the vote scheduling logic
pub trait VoteScheduler<OrgId, ShareId, VoteId>:
    SetDefaultShareApprovalOrder<OrgId, ShareId>
{
    type VoteSchedule: GetCurrentVoteIdentifiers<ShareId, VoteId>;

    fn dispatch_vote_schedule_from_vec_of_share_id(
        organization: OrgId,
        proposal_type: Self::ProposalType,
        share_ids: Vec<ShareId>,
    ) -> Result<Self::VoteSchedule, DispatchError>;

    // moves the vote schedule to the next scheduled vote in the sequence
    fn move_to_next_scheduled_vote(
        organization: OrgId,
        schedule: Self::VoteSchedule,
    ) -> Result<Option<Self::VoteSchedule>, DispatchError>;
}

pub trait ScheduleVoteSequence<OrgId, ShareId, VoteId, FineArithmetic>:
    VoteScheduleBuilder<OrgId, ShareId, FineArithmetic>
{
    type ProposalIndex;

    // this returns the first `VoteId` and stores the rest in a vote schedule in storage
    fn schedule_default_vote_schedule_for_proposal_type(
        organization: OrgId,
        index: Self::ProposalIndex,
        proposal_type: Self::ProposalType,
        // if None, just use the default vote schedule
        custom_share_ids: Option<Vec<ShareId>>,
    ) -> Result<VoteId, DispatchError>;
}

/// Checks the progress of a scheduled vote sequence and pushes the schedule along
/// - this should be called every `T::PollingFrequency::get()` number of blocks in `on_finalize`
pub trait PollActiveProposal<OrgId, ShareId, VoteId, FineArithmetic>:
    ScheduleVoteSequence<OrgId, ShareId, VoteId, FineArithmetic>
{
    type PollingOutcome;
    // This method checks the outcome of the current vote and moves the schedule to the next one when the threshold is met
    // - returns the newest `VoteId` when the voting schedule is pushed to the next vote
    fn poll_active_proposal(
        organization: OrgId,
        index: Self::ProposalIndex,
    ) -> Result<Self::PollingOutcome, DispatchError>;
}

// ====== Permissions ACL (in Bank) ======

pub trait SudoKeyManagement<AccountId> {
    fn is_sudo_key(who: &AccountId) -> bool;
    // cas
    fn swap_sudo_key(old_key: AccountId, new_key: AccountId) -> Result<AccountId, DispatchError>;
}

pub trait SupervisorKeyManagement<OrgId, AccountId>: SudoKeyManagement<AccountId> {
    fn is_organization_supervisor(organization: OrgId, who: &AccountId) -> bool;
    // cas, but also include the sudo as the possible old_key input for acl purposes
    fn swap_supervisor(
        organization: OrgId,
        old_key: AccountId,
        new_key: AccountId,
    ) -> Result<AccountId, DispatchError>;
}
