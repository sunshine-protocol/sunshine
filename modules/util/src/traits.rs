use codec::FullCodec;
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize},
    DispatchError, DispatchResult,
};
use sp_std::fmt::Debug;

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
    type GenesisAllocation: VerifyShape;

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

pub trait OpenVote {
    type VoteId;
    type ShareId;
    type ProposalType;

    fn open_vote(
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
        proposal_type: Self::ProposalType,
    ) -> Result<Self::VoteId, DispatchError>;
}

pub trait SetThresholdConfig<FineArithmetic>: OpenVote {
    type ThresholdConfig;

    fn set_threshold_config(
        share_id: Self::ShareId,
        proposal_type: Self::ProposalType,
        passage_threshold_pct: FineArithmetic,
        turnout_threshold_pct: FineArithmetic,
    ) -> DispatchResult;
}

pub trait CalculateVoteThreshold<Signal, FineArithmetic>:
    SetThresholdConfig<FineArithmetic>
{
    type VoteThreshold;

    fn calculate_vote_threshold(
        threshold_config: Self::ThresholdConfig,
        possible_turnout: Signal,
    ) -> Self::VoteThreshold;
}

pub trait MintableSignal<AccountId, Signal>: OpenVote {
    fn mint_signal(
        who: AccountId,
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
        amount: Option<Signal>,
    ) -> Result<Signal, DispatchError>;

    /// Mints signal for all ShareIds
    /// - calls ShareBank::shareholder_membership
    fn batch_mint_signal(
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
    ) -> Result<Signal, DispatchError>;
}

pub trait BurnableSignal<AccountId, Signal>: MintableSignal<AccountId, Signal> {
    fn burn_signal(
        who: &AccountId,
        vote_id: Self::VoteId,
        share_id: Self::ShareId,
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
pub trait VoteOnProposal<AccountId>: OpenVote + CheckVoteStatus {
    type Direction;
    type Magnitude;

    fn vote_on_proposal(
        voter: AccountId,
        vote_id: Self::VoteId,
        direction: Self::Direction,
        magnitude: Option<Self::Magnitude>,
    ) -> DispatchResult;
}
