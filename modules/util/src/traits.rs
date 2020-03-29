use crate::proposal::ProposalType;
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

/// For the module, to abstract the share reservation behavior
pub trait ReservableProfile<AccountId> {
    type Shares: AtLeast32Bit + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    type ShareId: AtLeast32Bit
        + FullCodec
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Default
        + From<u32>;

    /// Reserves amount iff `who` can afford to do so
    fn reserve(
        who: &AccountId,
        share_id: Self::ShareId,
        amount: Option<Self::Shares>,
    ) -> Result<Self::Shares, DispatchError>;
    /// Unreserves amount iff `who` can afford to do so
    fn unreserve(
        who: &AccountId,
        share_id: Self::ShareId,
        amount: Option<Self::Shares>,
    ) -> Result<Self::Shares, DispatchError>;
}

/// For testing purposes only; not for production because we don't want to duplicate calls to storage in other methods
pub trait GetProfile<AccountId>: ReservableProfile<AccountId> {
    fn get_free_shares(
        who: &AccountId,
        share_id: Self::ShareId,
    ) -> Result<Self::Shares, DispatchError>;
    fn get_reserved_shares(
        who: &AccountId,
        share_id: Self::ShareId,
    ) -> Result<Self::Shares, DispatchError>;
}

pub trait VerifyShape {
    // required bound on GenesisAllocation
    fn verify_shape(&self) -> bool;
}

/// For the module, to encode share registration behavior
pub trait ShareRegistration<AccountId>: ReservableProfile<AccountId> {
    type GenesisAllocation: From<Vec<(AccountId, Self::Shares)>> + VerifyShape;

    fn register(
        proposed_id: Self::ShareId,
        genesis_allocation: Self::GenesisAllocation,
    ) -> Result<Self::ShareId, DispatchError>;
}

/// For the module, to separate the issuance logic for shares
pub trait ShareBank<AccountId>: ShareRegistration<AccountId> {
    fn outstanding_shares(id: Self::ShareId) -> Self::Shares;
    // return membership group associated with a share type
    fn shareholder_membership(id: Self::ShareId) -> Result<Vec<AccountId>, DispatchError>;
    // TODO: replace amount with ownership table
    fn issue(owner: &AccountId, id: Self::ShareId, amount: Self::Shares) -> DispatchResult;
    // TODO: replace amount with ownership table
    fn buyback(owner: &AccountId, id: Self::ShareId, amount: Self::Shares) -> DispatchResult;
}

// ====== Vote Logic ======

pub trait GetVoteOutcome {
    type VoteId: AtLeast32Bit + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;
    type Outcome: Approved;
    fn get_vote_outcome(vote_id: Self::VoteId) -> Result<Self::Outcome, DispatchError>;
}

pub trait OpenVote<AccountId, FineArithmetic>: GetVoteOutcome {
    type ShareRegistrar: ShareRegistration<AccountId>;
    type ProposalType: From<ProposalType>;

    fn open_vote(
        vote_id: Self::VoteId,
        share_id: <Self::ShareRegistrar as ReservableProfile<AccountId>>::ShareId,
        proposal_type: Self::ProposalType,
        passage_threshold_pct: FineArithmetic,
        turnout_threshold_pct: FineArithmetic,
    ) -> Result<Self::VoteId, DispatchError>;
}

use crate::vote::ThresholdConfig;

// to be reused in other voting modules with different threshold config objects
pub trait VoteThresholdBuilder<AccountId, Signal, FineArithmetic>:
    OpenVote<AccountId, FineArithmetic>
{
    type ThresholdConfig: Into<ThresholdConfig<FineArithmetic>>;
    type VoteThreshold;

    fn build_vote_threshold(
        threshold_config: Self::ThresholdConfig,
        possible_turnout: Signal,
    ) -> Self::VoteThreshold;
}

pub trait MintableSignal<AccountId, FineArithmetic, Signal>:
    OpenVote<AccountId, FineArithmetic>
{
    fn mint_signal(
        who: AccountId,
        vote_id: Self::VoteId,
        share_id: <Self::ShareRegistrar as ReservableProfile<AccountId>>::ShareId,
        amount: Option<Signal>,
    ) -> Result<Signal, DispatchError>;

    /// Mints signal for all ShareIds
    /// - calls ShareBank::shareholder_membership
    fn batch_mint_signal(
        vote_id: Self::VoteId,
        share_id: <Self::ShareRegistrar as ReservableProfile<AccountId>>::ShareId,
    ) -> Result<Signal, DispatchError>;
}

pub trait BurnableSignal<AccountId, FineArithmetic, Signal>:
    MintableSignal<AccountId, FineArithmetic, Signal>
{
    fn burn_signal(
        who: &AccountId,
        vote_id: Self::VoteId,
        share_id: <Self::ShareRegistrar as ReservableProfile<AccountId>>::ShareId,
        amount: Option<Signal>,
    ) -> DispatchResult;
}

/// For VoteState, to verify passage
pub trait Approved {
    fn approved(&self) -> bool;
}

/// For module to apply the vote in the context of the existing module instance
pub trait ApplyVote {
    type Vote;
    type State: Approved;

    fn apply_vote(state: Self::State, vote: Self::Vote) -> Result<Self::State, DispatchError>;
}

/// For the module to check the status of the vote in the context of the existing module instance
pub trait CheckVoteStatus: ApplyVote {
    type Outcome;

    fn check_vote_outcome(state: Self::State) -> Result<Self::Outcome, DispatchError>;
    fn check_vote_expired(state: Self::State) -> bool;
}

/// For module to update vote state
pub trait VoteOnProposal<AccountId, FineArithmetic>:
    OpenVote<AccountId, FineArithmetic> + CheckVoteStatus
{
    type Direction;
    type Magnitude;

    fn vote_on_proposal(
        voter: AccountId,
        vote_id: Self::VoteId,
        direction: Self::Direction,
        magnitude: Option<Self::Magnitude>,
    ) -> DispatchResult;
}

// ====== Vote Dispatch Logic (in Bank) ======

pub trait SetDefaultShareApprovalOrder<ShareId, OrgId> {
    type ProposalType;

    fn set_default_share_approval_order_for_proposal_type(
        organization: OrgId,
        proposal_type: Self::ProposalType,
        share_approval_order: Vec<ShareId>,
    ) -> DispatchResult;
}

pub trait ScheduledVoteBuilder<ShareId, OrgId, FineArithmetic>:
    SetDefaultShareApprovalOrder<ShareId, OrgId>
{
    type ScheduledVote;

    fn set_share_id_proposal_type_to_threshold(
        organization: OrgId,
        share_id: ShareId,
        proposal_type: Self::ProposalType,
        passage_threshold_pct: FineArithmetic,
        turnout_threshold_pct: FineArithmetic,
    ) -> DispatchResult;

    fn scheduled_vote_from_share_id_proposal_type(
        organization: OrgId,
        share_id: ShareId,
        proposal_type: Self::ProposalType,
    ) -> Result<Self::ScheduledVote, DispatchError>;
}

pub trait SetDefaultVoteSchedule<ShareId, OrgId, FineArithmetic>:
    ScheduledVoteBuilder<ShareId, OrgId, FineArithmetic>
{
    fn set_default_vote_schedule_for_proposal_type(
        organization: OrgId,
        proposal_type: Self::ProposalType,
        // if None, use the default share approval order
        raw_vote_schedule: Option<Vec<Self::ScheduledVote>>,
    ) -> DispatchResult;
}

pub trait ScheduleDefaultVoteSchedule<ShareId, VoteId, OrgId, FineArithmetic>:
    SetDefaultVoteSchedule<ShareId, OrgId, FineArithmetic>
{
    type ProposalIndex;

    // this returns the first `VoteId` and stores the rest in a vote schedule in storage
    fn schedule_default_vote_schedule_for_proposal_type(
        organization: OrgId,
        index: Self::ProposalIndex,
        proposal_type: Self::ProposalType,
        custom_share_ids: Option<Vec<ShareId>>,
    ) -> Result<VoteId, DispatchError>;
}

// TODO: implement on module
// this should be more permissioned than the default vote schedule
pub trait ScheduleCustomVoteSequence<ShareId, VoteId, OrgId, FineArithmetic>:
    ScheduleDefaultVoteSchedule<ShareId, VoteId, OrgId, FineArithmetic>
{
    fn schedule_custom_vote_sequence_for_proposal_type(
        organization: OrgId,
        index: Self::ProposalIndex,
        proposal_type: Self::ProposalType,
        custom_vote_sequence: Vec<Self::ScheduledVote>,
    ) -> Result<VoteId, DispatchError>;
}

/// Checks the progress of a scheduled vote sequence and pushes the schedule along
/// - this should be called every `T::PollingFrequency::get()` number of blocks in `on_finalize`
pub trait PollActiveProposal<ShareId, VoteId, OrgId, FineArithmetic>:
    ScheduleDefaultVoteSchedule<ShareId, VoteId, OrgId, FineArithmetic>
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

pub trait SupervisorKeyManagement<AccountId, OrgId>: SudoKeyManagement<AccountId> {
    fn is_organization_supervisor(organization: OrgId, who: &AccountId) -> bool;
    // cas, but also include the sudo as the possible old_key input for acl purposes
    fn swap_supervisor(
        organization: OrgId,
        old_key: AccountId,
        new_key: AccountId,
    ) -> Result<AccountId, DispatchError>;
}
