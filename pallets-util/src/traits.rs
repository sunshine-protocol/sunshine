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

pub trait Increment: Sized {
    fn increment(self) -> Self;
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
    fn remove_organization(id: OrgId) -> Result<Option<Vec<OrgId>>>;
    fn recursive_remove_organization(id: OrgId) -> DispatchResult;
}

// ====== Vote Logic ======

/// Retrieves the outcome of a vote associated with the vote identifier `vote_id`
pub trait GetVoteOutcome<VoteId> {
    type Outcome;

    fn get_vote_outcome(vote_id: VoteId) -> Result<Self::Outcome>;
}

/// Open a new vote for the organization, share_id and a custom threshold requirement
pub trait OpenVote<OrgId, Threshold, BlockNumber, Hash> {
    type VoteIdentifier;
    fn open_vote(
        topic: Option<Hash>,
        organization: OrgId,
        passage_threshold: Threshold,
        rejection_threshold: Option<Threshold>,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_unanimous_consent(
        topic: Option<Hash>,
        organization: OrgId,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
}

pub trait OpenThresholdVote<OrgId, Threshold, BlockNumber, Hash, FineArithmetic>:
    OpenVote<OrgId, Threshold, BlockNumber, Hash>
{
    const THIRTY_FOUR_PERCENT: FineArithmetic;
    const FIFTY_ONE_PERCENT: FineArithmetic;
    const SIXTY_SEVEN_PERCENT: FineArithmetic;
    const SEVENTY_SIX_PERCENT: FineArithmetic;
    const NINETY_ONE_PERCENT: FineArithmetic;
    fn open_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
        passage_threshold_pct: FineArithmetic,
        rejection_threshold_pct: Option<FineArithmetic>,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_34_pct_passage_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_51_pct_passage_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_67_pct_passage_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_76_pct_passage_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteIdentifier>;
    fn open_91_pct_passage_threshold_vote(
        topic: Option<Hash>,
        organization: OrgId,
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

pub trait MintableSignal<AccountId, OrgId, Threshold, BlockNumber, VoteId, Hash>:
    OpenVote<OrgId, Threshold, BlockNumber, Hash> + ApplyVote<Hash>
{
    fn mint_custom_signal_for_account(
        vote_id: VoteId,
        who: &AccountId,
        signal: Self::Signal,
    );
    fn batch_mint_equal_signal(
        vote_id: VoteId,
        organization: OrgId,
    ) -> Result<Self::Signal>;
    fn batch_mint_signal(
        vote_id: VoteId,
        organization: OrgId,
    ) -> Result<Self::Signal>;
}

/// Define the rate at which signal is burned to unreserve shares in an organization
pub trait BurnableSignal<AccountId, OrgId, Threshold, BlockNumber, VoteId, Hash>:
    MintableSignal<AccountId, OrgId, Threshold, BlockNumber, VoteId, Hash>
{
    fn burn_signal(
        vote_id: VoteId,
        who: &AccountId,
        amount: Option<Self::Signal>, // if None, then all
    ) -> DispatchResult;
}

pub trait VoteOnProposal<AccountId, OrgId, Threshold, BlockNumber, VoteId, Hash>:
    OpenVote<OrgId, Threshold, BlockNumber, Hash> + CheckVoteStatus<Hash, VoteId>
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

// ~~~~~~~~ Bounty Module ~~~~~~~~

pub trait ReturnsBountyIdentifier {
    type BountyId;
}

pub trait PostBounty<
    AccountId,
    OrgId,
    SpendableBank,
    Currency,
    Hash,
    ReviewCommittee,
>: ReturnsBountyIdentifier
{
    type BountyInfo;
    fn post_bounty(
        poster: AccountId,
        on_behalf_of: Option<SpendableBank>,
        description: Hash,
        amount_reserved_for_bounty: Currency,
        acceptance_committee: ReviewCommittee,
        supervision_committee: Option<ReviewCommittee>,
    ) -> Result<Self::BountyId>;
}
// TODO: make an issue for this and prefer an impl that presents a multisig
pub trait UseTermsOfAgreement<OrgId> {
    type VoteIdentifier;
    fn request_consent_on_terms_of_agreement(
        team_org: OrgId,
    ) -> Result<Self::VoteIdentifier>;
}
pub trait StartTeamConsentPetition<VoteIdentifier>: Sized {
    fn start_team_consent_petition(
        &self,
        vote_id: VoteIdentifier,
    ) -> Option<Self>;
    fn get_team_consent_vote_id(&self) -> Option<VoteIdentifier>;
}

pub trait StartReview<VoteIdentifier>: Sized {
    fn start_review(&self, vote_id: VoteIdentifier) -> Option<Self>;
    fn get_review_id(&self) -> Option<VoteIdentifier>;
}

pub trait ApproveWithoutTransfer: Sized {
    // infallible
    fn approve_without_transfer(&self) -> Self;
}

pub trait ApproveGrant: Sized {
    fn approve_grant(&self) -> Self;
    fn grant_approved(&self) -> bool;
}
// TODO: RevokeApprovedGrant<VoteID> => vote to take away the team's grant and clean storage

pub trait SpendApprovedGrant<Currency>: Sized {
    fn spend_approved_grant(&self, amount: Currency) -> Option<Self>;
}

pub trait SubmitGrantApplication<AccountId, VoteId, BankId, Currency, Hash>:
    ReturnsBountyIdentifier
{
    type GrantApp: StartReview<VoteId> + ApproveGrant; //+ StartTeamConsentPetition<VoteId>
    fn submit_grant_application(
        submitter: AccountId,
        bank: Option<BankId>,
        bounty_id: Self::BountyId,
        description: Hash,
        total_amount: Currency,
    ) -> Result<Self::BountyId>; // returns application identifier
}

pub trait SuperviseGrantApplication<BountyId, AccountId> {
    type AppState;
    fn trigger_application_review(
        bounty_id: BountyId,
        application_id: BountyId,
    ) -> Result<Self::AppState>;
    fn sudo_approve_application(
        sudo: AccountId,
        bounty_id: BountyId,
        application_id: BountyId,
    ) -> Result<Self::AppState>;
    fn poll_application(
        bounty_id: BountyId,
        application_id: BountyId,
    ) -> Result<Self::AppState>;
}

pub trait SubmitMilestone<AccountId, BountyId, Hash, Currency, VoteId, BankId> {
    type Milestone: StartReview<VoteId> + ApproveWithoutTransfer;
    type MilestoneState;
    fn submit_milestone(
        submitter: AccountId,
        bounty_id: BountyId,
        application_id: BountyId,
        submission_reference: Hash,
        amount_requested: Currency,
    ) -> Result<BountyId>; // returns milestone identifier
    fn trigger_milestone_review(
        bounty_id: BountyId,
        milestone_id: BountyId,
    ) -> Result<Self::MilestoneState>;
    fn sudo_approves_milestone(
        caller: AccountId,
        bounty_id: BountyId,
        milestone_id: BountyId,
    ) -> Result<Self::MilestoneState>;
    fn poll_milestone(
        bounty_id: BountyId,
        milestone_id: BountyId,
    ) -> Result<Self::MilestoneState>;
}

// We could remove`can_submit_grant_app` or `can_submit_milestone` because both of these paths log the submitter
// in the associated state anyway so we might as well pass the caller into the methods that do this logic and
// perform any context-based authentication there, but readability is more important at this point
pub trait BountyPermissions<OrgId, TermsOfAgreement, AccountId, BountyId>:
    UseTermsOfAgreement<OrgId>
{
    fn can_create_bounty(who: &AccountId, hosting_org: OrgId) -> bool;
    fn can_submit_grant_app(who: &AccountId, terms: TermsOfAgreement) -> bool;
    fn can_trigger_grant_app_review(
        who: &AccountId,
        bounty_id: BountyId,
    ) -> Result<bool>;
    fn can_poll_grant_app(who: &AccountId, bounty_id: BountyId)
        -> Result<bool>;
    fn can_submit_milestone(
        who: &AccountId,
        bounty_id: BountyId,
        application_id: BountyId,
    ) -> Result<bool>;
    fn can_poll_milestone(who: &AccountId, bounty_id: BountyId)
        -> Result<bool>;
    fn can_trigger_milestone_review(
        who: &AccountId,
        bounty_id: BountyId,
    ) -> Result<bool>;
}
