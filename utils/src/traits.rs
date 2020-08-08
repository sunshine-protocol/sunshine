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
    fn account_ownership(&self) -> Vec<(AccountId, Shares)>;
}
pub trait AccessProfile<Shares> {
    fn total(&self) -> Shares;
}
use crate::share::SimpleShareGenesis;
pub trait ShareInformation<OrgId, AccountId, Shares> {
    type Profile: AccessProfile<Shares>;
    type Genesis: From<Vec<(AccountId, Shares)>>
        + Into<SimpleShareGenesis<AccountId, Shares>>
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
    ) -> DispatchResult;
    fn batch_issue(
        organization: OrgId,
        genesis: Self::Genesis,
    ) -> DispatchResult;
    fn batch_burn(
        organization: OrgId,
        genesis: Self::Genesis,
    ) -> DispatchResult;
}
pub trait ReserveProfile<OrgId, AccountId, Shares>:
    ShareIssuance<OrgId, AccountId, Shares>
{
    fn reserve(
        organization: OrgId,
        who: &AccountId,
        amount: Option<Shares>,
    ) -> Result<Shares>;
    fn unreserve(
        organization: OrgId,
        who: &AccountId,
        amount: Option<Shares>,
    ) -> Result<Shares>;
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
        parent_id: Option<OrgId>,
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

pub trait UpdateVoteTopic<VoteId, Hash> {
    fn update_vote_topic(
        vote_id: VoteId,
        new_topic: Hash,
        clear_previous_vote_state: bool,
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
    fn mint_custom_signal_for_account(
        vote_id: VoteId,
        who: &AccountId,
        signal: Signal,
    );
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

pub trait BankPermissions<BankId, OrgId, AccountId> {
    fn can_open_bank_account_for_org(org: OrgId, who: &AccountId) -> bool;
    fn can_propose_spend(bank: BankId, who: &AccountId) -> Result<bool>;
    fn can_trigger_vote_on_spend_proposal(
        bank: BankId,
        who: &AccountId,
    ) -> Result<bool>;
    fn can_sudo_approve_spend_proposal(
        bank: BankId,
        who: &AccountId,
    ) -> Result<bool>;
    fn can_poll_spend_proposal(bank: BankId, who: &AccountId) -> Result<bool>;
    fn can_spend(bank: BankId, who: &AccountId) -> Result<bool>;
}

pub trait OpenBankAccount<OrgId, Currency, AccountId> {
    type BankId;
    fn open_bank_account(
        opener: AccountId,
        org: OrgId,
        deposit: Currency,
        controller: Option<AccountId>,
    ) -> Result<Self::BankId>;
}

pub trait SpendGovernance<BankId, Currency, AccountId> {
    type SpendId;
    type VoteId;
    type SpendState;
    fn propose_spend(
        bank_id: BankId,
        amount: Currency,
        dest: AccountId,
    ) -> Result<Self::SpendId>;
    fn trigger_vote_on_spend_proposal(
        spend_id: Self::SpendId,
    ) -> Result<Self::VoteId>;
    fn sudo_approve_spend_proposal(spend_id: Self::SpendId) -> DispatchResult;
    fn poll_spend_proposal(spend_id: Self::SpendId)
        -> Result<Self::SpendState>;
}
