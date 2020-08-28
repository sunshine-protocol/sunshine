use sp_runtime::{
    DispatchError,
    DispatchResult,
};
use sp_std::prelude::*;

pub type Result<T> = sp_std::result::Result<T, DispatchError>;

// === Unique ID Logic, Useful for All Modules ===

pub trait IDIsAvailable<Id> {
    fn id_is_available(id: Id) -> bool;
}

pub trait GenerateUniqueID<Id> {
    fn generate_unique_id() -> Id;
}

pub trait SeededGenerateUniqueID<Id, Seed> {
    fn seeded_generate_unique_id(seed: Seed) -> Id;
}

// ====== Permissions ACL ======

pub trait OrganizationSupervisorPermissions<OrgId, AccountId> {
    fn is_organization_supervisor(org: OrgId, who: &AccountId) -> bool;
    // removes any existing sudo and places None
    fn clear_organization_supervisor(org: OrgId) -> DispatchResult;
    // removes any existing sudo and places `who`
    fn put_organization_supervisor(
        org: OrgId,
        who: AccountId,
    ) -> DispatchResult;
}

// ---------- Membership Logic ----------

/// Checks that the `AccountId` is a member of a share group in an organization
pub trait GroupMembership<OrgId, AccountId> {
    fn is_member_of_group(org_id: OrgId, who: &AccountId) -> bool;
}
use orml_utilities::OrderedSet;
pub trait GetGroup<OrgId, AccountId> {
    fn get_group(organization: OrgId) -> Option<OrderedSet<AccountId>>;
}
/// Checks that the `total` field is correct by summing all assigned share quantities
pub trait VerifyShape {
    // required bound on GenesisAllocation
    fn verify_shape(&self) -> bool;
}
pub trait AccessGenesis<AccountId, Shares> {
    fn total(&self) -> Shares;
    fn vec(&self) -> Vec<(AccountId, Shares)>;
}
pub trait AccessProfile<Shares> {
    fn total(&self) -> Shares;
}
use crate::share::WeightedVector;
pub trait ShareInformation<OrgId, AccountId, Shares> {
    type Profile: AccessProfile<Shares>;
    type Genesis: From<Vec<(AccountId, Shares)>>
        + Into<WeightedVector<AccountId, Shares>>
        + VerifyShape
        + AccessGenesis<AccountId, Shares>;
    /// Gets the total number of shares issued for an organization's share identifier
    fn outstanding_shares(organization: OrgId) -> Shares;
    // get who's share profile
    fn get_share_profile(
        organization: OrgId,
        who: &AccountId,
    ) -> Option<Self::Profile>;
    /// Returns the entire membership group associated with a share identifier, fallible bc checks existence
    fn get_membership_with_shape(organization: OrgId) -> Option<Self::Genesis>;
}
pub trait ShareIssuance<OrgId, AccountId, Shares>:
    ShareInformation<OrgId, AccountId, Shares>
{
    type Proportion;
    fn issue(
        organization: OrgId,
        new_owner: AccountId,
        amount: Shares,
        batch: bool,
    ) -> DispatchResult;
    fn burn(
        organization: OrgId,
        old_owner: AccountId,
        amount: Option<Shares>, // default None => burn all shares
        batch: bool,
    ) -> Result<Self::Proportion>;
    fn batch_issue(
        organization: OrgId,
        genesis: Self::Genesis,
    ) -> Result<Shares>;
    fn batch_burn(
        organization: OrgId,
        genesis: Self::Genesis,
    ) -> DispatchResult;
}
pub trait LockProfile<OrgId, AccountId> {
    fn lock_profile(organization: OrgId, who: &AccountId) -> DispatchResult;
    fn unlock_profile(organization: OrgId, who: &AccountId) -> DispatchResult;
}
pub trait RegisterOrganization<OrgId, AccountId, Hash> {
    type OrgSrc;
    type OrganizationState;
    // called to form the organization in the method below
    fn organization_from_src(
        src: Self::OrgSrc,
        org_id: OrgId,
        supervisor: Option<AccountId>,
        value_constitution: Hash,
    ) -> Result<Self::OrganizationState>;
    fn register_organization(
        source: Self::OrgSrc,
        supervisor: Option<AccountId>,
        value_constitution: Hash,
    ) -> Result<OrgId>; // returns OrgId in this module's context
    fn register_sub_organization(
        parent_id: OrgId,
        source: Self::OrgSrc,
        supervisor: Option<AccountId>,
        value_constitution: Hash,
    ) -> Result<OrgId>;
}
pub trait RemoveOrganization<OrgId> {
    // returns Ok(Some(child_id)) or Ok(None) if leaf org
    fn remove_organization(id: OrgId) -> DispatchResult;
    fn recursive_remove_organization(id: OrgId) -> DispatchResult;
}

// ====== Vote Logic ======

/// Retrieves the outcome of a vote associated with the vote identifier `vote_id`
pub trait GetVoteOutcome<VoteId> {
    type Outcome;

    fn get_vote_outcome(vote_id: VoteId) -> Result<Self::Outcome>;
}

/// Open a new vote for the organization, share_id and a custom threshold requirement
pub trait OpenVote<OrgId, Signal, Percent, BlockNumber, Hash> {
    type VoteIdentifier;
    fn open_vote(
        topic: Option<Hash>,
        organization: OrgId,
        threshold: Signal,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_percent_vote(
        topic: Option<Hash>,
        organization: OrgId,
        threshold: Percent,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
}

pub trait ConfigureThreshold<Threshold, Hash, BlockNumber> {
    type ThresholdId;
    type VoteId; // TODO: make this same as OpenVote type by merging traits someday somehow
    fn register_threshold(t: Threshold) -> Result<Self::ThresholdId>;
    fn invoke_threshold(
        id: Self::ThresholdId,
        topic: Option<Hash>,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteId>;
}

pub trait UpdateVote<VoteId, Hash, BlockNumber> {
    fn update_vote_topic(
        vote_id: VoteId,
        new_topic: Hash,
        clear_previous_vote_state: bool,
    ) -> DispatchResult;
    fn extend_vote_length(
        vote_id: VoteId,
        blocks_from_now: BlockNumber,
    ) -> DispatchResult;
}

pub trait Approved {
    fn approved(&self) -> bool;
}
pub trait Rejected {
    fn rejected(&self) -> Option<bool>;
}
pub trait Apply<Signal, View>: Sized {
    fn apply(
        &self,
        magnitude: Signal,
        old_direction: View,
        new_direction: View,
    ) -> Option<Self>;
}

pub trait VoteVector<Signal, Direction, Hash> {
    fn magnitude(&self) -> Signal;
    fn direction(&self) -> Direction;
    fn justification(&self) -> Option<Hash>;
}

pub trait ApplyVote<Hash> {
    type Signal;
    type Direction;
    type Vote: VoteVector<Self::Signal, Self::Direction, Hash>;
    type State: Approved + Apply<Self::Signal, Self::Direction>;
    fn apply_vote(
        state: Self::State,
        vote_magnitude: Self::Signal,
        old_vote_view: Self::Direction,
        new_vote_view: Self::Direction,
    ) -> Option<Self::State>;
}

pub trait CheckVoteStatus<Hash, VoteId>:
    ApplyVote<Hash> + GetVoteOutcome<VoteId>
{
    fn check_vote_expired(state: &Self::State) -> bool;
}

pub trait MintableSignal<AccountId, OrgId, VoteId, Signal> {
    fn batch_mint_equal_signal(
        vote_id: VoteId,
        organization: OrgId,
    ) -> Result<Signal>;
    fn batch_mint_signal(
        vote_id: VoteId,
        organization: OrgId,
    ) -> Result<Signal>;
}

pub trait VoteOnProposal<AccountId, VoteId, Hash>:
    CheckVoteStatus<Hash, VoteId>
{
    fn vote_on_proposal(
        vote_id: VoteId,
        voter: AccountId,
        direction: Self::Direction,
        justification: Option<Hash>,
    ) -> DispatchResult;
}

// ====== Court Logic ======

pub trait RegisterDisputeType<AccountId, Currency, VoteMetadata, BlockNumber> {
    type DisputeIdentifier;
    fn register_dispute_type(
        locker: AccountId,
        amount_to_lock: Currency,
        dispute_raiser: AccountId,
        resolution_path: VoteMetadata,
        expiry: Option<BlockNumber>,
    ) -> Result<Self::DisputeIdentifier>;
}

// ~~~~~~~~ Bank Module ~~~~~~~~

pub trait OpenBankAccount<OrgId, Currency, AccountId, Threshold> {
    type BankId;
    fn open_bank_account(
        opener: AccountId,
        org: OrgId,
        deposit: Currency,
        controller: Option<AccountId>,
        threshold: Threshold,
    ) -> Result<Self::BankId>;
}

pub trait SpendGovernance<BankId, Currency, AccountId, SProp> {
    type SpendId;
    type VoteId;
    type SpendState;
    fn _propose_spend(
        caller: &AccountId,
        bank_id: BankId,
        amount: Currency,
        dest: AccountId,
    ) -> Result<Self::SpendId>;
    fn _trigger_vote_on_spend_proposal(
        caller: &AccountId,
        bank_id: BankId,
        spend_id: Self::SpendId,
    ) -> Result<Self::VoteId>;
    fn _sudo_approve_spend_proposal(
        caller: &AccountId,
        bank_id: BankId,
        spend_id: Self::SpendId,
    ) -> DispatchResult;
    fn poll_spend_proposal(prop: SProp) -> Result<Self::SpendState>;
}

// TODO: merge functionality with SpendGovernance
pub trait DocGovernance<CommitteeId, Cid, AccountId, Proposal> {
    type ProposalId;
    type VoteId;
    type PropState;
    type DocIndex;
    fn _propose_doc(
        caller: AccountId,
        committee_id: CommitteeId,
        doc_ref: Cid,
    ) -> Result<Self::ProposalId>;
    fn _trigger_vote_on_proposal(
        caller: &AccountId,
        committee_id: CommitteeId,
        proposal_id: Self::ProposalId,
    ) -> Result<(Cid, Self::VoteId)>;
    fn _sudo_approve_proposal(
        caller: &AccountId,
        committee_id: CommitteeId,
        proposal_id: Self::ProposalId,
    ) -> Result<(Self::DocIndex, Cid)>;
    fn poll_proposal(prop: Proposal) -> Result<Self::PropState>;
}

pub trait MolochMembership<AccountId, BankId, Currency, Shares, MProp> {
    type MemberPropId;
    type VoteId;
    type PropState;
    fn _propose_member(
        caller: &AccountId,
        bank_id: BankId,
        tribute: Currency,
        shares_requested: Shares,
        applicant: AccountId,
    ) -> Result<Self::MemberPropId>;
    fn _trigger_vote_on_member_proposal(
        caller: &AccountId,
        bank_id: BankId,
        proposal_id: Self::MemberPropId,
    ) -> Result<Self::VoteId>;
    fn poll_membership_proposal(prop: MProp) -> Result<Self::PropState>;
    fn _burn_shares(caller: AccountId, bank_id: BankId) -> DispatchResult;
}
