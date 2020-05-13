use crate::{
    schedule::ThresholdConfigBuilder,
    share::SimpleShareGenesis,
    voteyesno::{SupportedVoteTypes, ThresholdConfig},
};
use codec::Codec;
use frame_support::Parameter;
use sp_runtime::{
    traits::{AtLeast32Bit, Member},
    DispatchError, DispatchResult, PerThing,
};
use sp_std::prelude::*;

// === Unique ID Logic, Useful for All Modules ===

/// For the module to implement for its id type (typically a common double_map prefix key)
pub trait IDIsAvailable<Id> {
    fn id_is_available(id: Id) -> bool;
}

/// For the module to implement for its id type (typically a common double_map prefix key)
pub trait GenerateUniqueID<Id>: IDIsAvailable<Id> {
    // this should be infallible, it returns the generated unique id which may or may not be equal to the original value
    fn generate_unique_id(proposed_id: Id) -> Id;
}

// ====== Permissions ACL ======

pub trait ChainSudoPermissions<AccountId> {
    fn is_sudo_key(who: &AccountId) -> bool;
    // infallible, unguarded
    fn put_sudo_key(who: AccountId);
    // fallible, cas by default
    fn set_sudo_key(old_key: &AccountId, new_key: AccountId) -> DispatchResult;
}

pub trait OrganizationSupervisorPermissions<OrgId, AccountId> {
    fn is_organization_supervisor(org: OrgId, who: &AccountId) -> bool;
    // infallible
    fn put_organization_supervisor(org: OrgId, who: AccountId);
    // fallible, cas by default
    fn set_organization_supervisor(
        org: OrgId,
        old_supervisor: &AccountId,
        new_supervisor: AccountId,
    ) -> DispatchResult;
}

pub trait SubGroupSupervisorPermissions<OrgId, S1, AccountId> {
    fn is_sub_group_supervisor(org: OrgId, sub_group: S1, who: &AccountId) -> bool;
    // infallible
    fn put_sub_group_supervisor(org: OrgId, sub_group: S1, who: AccountId);
    // fallible, case by default
    fn set_sub_group_supervisor(
        org: OrgId,
        sub_group: S1,
        old_supervisor: &AccountId,
        new_supervisor: AccountId,
    ) -> DispatchResult;
}

pub trait SubSubGroupSupervisorPermissions<OrgId, S1, S2, AccountId> {
    fn is_sub_sub_group_organization_supervisor(
        org: OrgId,
        sub_group: S1,
        sub_sub_group: S2,
        who: &AccountId,
    ) -> bool;
    // infallible
    fn put_sub_sub_group_organization_supervisor(
        org: OrgId,
        sub_group: S1,
        sub_sub_group: S2,
        who: AccountId,
    );
    // fallible, cas by default
    fn set_sub_sub_group_supervisor(
        org: OrgId,
        sub_group: S1,
        sub_sub_group: S2,
        old_supervisor: &AccountId,
        new_supervisor: AccountId,
    ) -> DispatchResult;
}

// ---------- Membership Logic ----------
pub trait GetGroupSize {
    type GroupId;

    fn get_size_of_group(group_id: Self::GroupId) -> u32;
}

/// Checks that the `AccountId` is a member of a share group in an organization
pub trait GroupMembership<AccountId>: GetGroupSize {
    fn is_member_of_group(group_id: Self::GroupId, who: &AccountId) -> bool;
}

/// All changes to the organizational membership are infallible
pub trait ChangeGroupMembership<AccountId>: GroupMembership<AccountId> {
    fn add_group_member(group_id: Self::GroupId, new_member: AccountId, batch: bool);
    fn remove_group_member(group_id: Self::GroupId, old_member: AccountId, batch: bool);
    /// WARNING: the vector fed as inputs to the following methods must have NO duplicates
    fn batch_add_group_members(group_id: Self::GroupId, new_members: Vec<AccountId>);
    fn batch_remove_group_members(group_id: Self::GroupId, old_members: Vec<AccountId>);
}
pub trait GetFlatShareGroup<AccountId> {
    fn get_organization_share_group(organization: u32, share_id: u32) -> Option<Vec<AccountId>>;
}
pub trait GetTotalShareIssuance<Shares> {
    fn get_total_share_issuance(organization: u32, share_id: u32) -> Result<Shares, DispatchError>;
}
pub trait GetWeightedShareGroupShape<AccountId, Shares>: GetTotalShareIssuance<Shares> {
    fn get_weighted_share_group_shape(
        organization: u32,
        share_id: u32,
    ) -> Result<Vec<(AccountId, Shares)>, DispatchError>;
}

// ---------- Petition Logic ----------

pub trait GetPetitionStatus {
    type Status; // says approvals gotten and outcome, including veto context?

    fn get_petition_status(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Result<Self::Status, DispatchError>;
}

pub trait OpenPetition<Hash, BlockNumber>: GetPetitionStatus {
    fn open_petition(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        topic: Hash,
        required_support: u32,
        require_against: Option<u32>,
        ends: Option<BlockNumber>,
    ) -> DispatchResult;
}

pub trait UpdatePetitionTerms<Hash>: Sized {
    fn update_petition_terms(&self, new_terms: Hash) -> Self;
}
pub trait Vetoed {
    fn vetoed(&self) -> bool;
}

pub trait SignPetition<AccountId, Hash> {
    type Petition: Approved
        + Vetoed
        + Rejected
        + UpdatePetitionTerms<Hash>
        + Apply<Self::SignerView>;
    type SignerView;
    type Outcome;
    fn sign_petition(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        signer: AccountId,
        view: Self::SignerView,
        justification: Hash,
    ) -> Result<Self::Outcome, DispatchError>;
}

pub trait EmpowerWithVeto<AccountId> {
    fn get_those_empowered_with_veto(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<AccountId>>;
    fn get_those_who_invoked_veto(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<AccountId>>;
    fn empower_with_veto(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        // if none, give it to everyone in the share_id group
        accounts: Option<Vec<AccountId>>,
    ) -> DispatchResult;
}

pub trait GetFullVetoContext<AccountId>: EmpowerWithVeto<AccountId> {
    type VetoContext;
    fn get_full_veto_context(
        organization: u32,
        share_id: u32,
        petition_id: u32,
    ) -> Option<Vec<(AccountId, Self::VetoContext)>>;
}

pub trait RequestChanges<AccountId, Hash>: SignPetition<AccountId, Hash> {
    fn request_changes(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        signer: AccountId,
        justification: Hash,
    ) -> Result<Option<Self::Outcome>, DispatchError>;
    fn accept_changes(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        signer: AccountId,
    ) -> Result<Option<Self::Outcome>, DispatchError>;
}

pub trait UpdatePetition<AccountId, Hash>: SignPetition<AccountId, Hash> {
    fn update_petition(
        organization: u32,
        share_id: u32,
        petition_id: u32,
        new_topic: Hash,
    ) -> Result<u32, DispatchError>;
} // do we need a delete petition when we want to close it

// ---------- Shares Atomic Logic ----------

/// Checks that the `total` field is correct by summing all assigned share quantities
pub trait VerifyShape {
    // required bound on GenesisAllocation
    fn verify_shape(&self) -> bool;
}
pub trait AccessGenesis<AccountId, Shares> {
    fn total(&self) -> Shares;
    fn account_ownership(&self) -> Vec<(AccountId, Shares)>;
}

pub trait WeightedShareGroup<AccountId> {
    type Shares: Parameter + Member + AtLeast32Bit + Codec;
    type Genesis: From<Vec<(AccountId, Self::Shares)>>
        + Into<SimpleShareGenesis<AccountId, Self::Shares>>
        + VerifyShape
        + AccessGenesis<AccountId, Self::Shares>;
    /// Gets the total number of shares issued for an organization's share identifier
    fn outstanding_shares(organization: u32, id: u32) -> Option<Self::Shares>;
    // get who's share profile
    fn get_share_profile(
        organization: u32,
        share_id: u32,
        who: &AccountId,
    ) -> Result<Self::Shares, DispatchError>;
    /// Returns the entire membership group associated with a share identifier, fallible bc checks existence
    fn shareholder_membership(organization: u32, id: u32) -> Option<Self::Genesis>;
}

/// Issuance logic for existing shares (not new shares)
pub trait ShareBank<AccountId>: WeightedShareGroup<AccountId> {
    fn issue(
        organization: u32,
        share_id: u32,
        new_owner: AccountId,
        amount: Self::Shares,
        batch: bool,
    ) -> DispatchResult;
    fn burn(
        organization: u32,
        share_id: u32,
        old_owner: AccountId,
        amount: Self::Shares,
        batch: bool,
    ) -> DispatchResult;
    fn batch_issue(organization: u32, share_id: u32, genesis: Self::Genesis) -> DispatchResult;
    fn batch_burn(organization: u32, share_id: u32, genesis: Self::Genesis) -> DispatchResult;
}

pub trait GetMagnitude<Shares> {
    fn get_magnitude(self) -> Shares;
}
// the first element is the number of times its been reserved
impl<Shares> GetMagnitude<Shares> for (u32, Shares) {
    fn get_magnitude(self) -> Shares {
        self.1
    }
}

/// Reserve shares for an individual `AccountId`
pub trait ReservableProfile<AccountId>: ShareBank<AccountId> {
    type ReservationContext: GetMagnitude<Self::Shares>;
    /// Reserves amount iff certain conditions are met wrt existing profile and how it will change
    fn reserve(
        organization: u32,
        share_id: u32,
        who: &AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError>;
    /// Unreserves amount iff certain conditions are met wrt existing profile and how it will change
    fn unreserve(
        organization: u32,
        share_id: u32,
        who: &AccountId,
        amount: Option<Self::ReservationContext>,
    ) -> Result<Self::ReservationContext, DispatchError>;
}

/// Lock shares for an individual `AccountId`
pub trait LockableProfile<AccountId> {
    fn lock_profile(organization: u32, share_id: u32, who: &AccountId) -> DispatchResult;
    fn unlock_profile(organization: u32, share_id: u32, who: &AccountId) -> DispatchResult;
}

// ====== Vote Logic ======

/// Retrieves the outcome of a vote associated with the vote identifier `vote_id`
pub trait GetVoteOutcome {
    type Signal: Parameter + Member + AtLeast32Bit + Codec;
    type Outcome: Approved;

    fn get_vote_outcome(
        organization: u32,
        share_id: u32,
        vote_id: u32,
    ) -> Result<Self::Outcome, DispatchError>;
}

/// Derives the threshold requirement from turnout (for `ThresholdConfig`)
pub trait DeriveThresholdRequirement<Signal> {
    fn derive_support_requirement(&self, turnout: Signal) -> Signal;
    fn derive_turnout_requirement(&self, turnout: Signal) -> Signal;
}

/// Checks that the `ThresholdConfig` that impls this method has both fields with the same `ThresholdType` variant
pub trait ConsistentThresholdStructure {
    fn is_percentage_threshold(&self) -> bool;
    fn is_count_threshold(&self) -> bool;
    fn has_consistent_structure(&self) -> bool;
}

/// Open a new vote for the organization, share_id and a custom threshold requirement
pub trait OpenShareGroupVote<AccountId, BlockNumber, FineArithmetic: PerThing>:
    GetVoteOutcome
{
    type ThresholdConfig: DeriveThresholdRequirement<Self::Signal>
        + ConsistentThresholdStructure
        + From<ThresholdConfig<Self::Signal, FineArithmetic>>
        + From<ThresholdConfigBuilder<FineArithmetic>>; // NOTE: this forces FineArithmetic generic parameter for traits and all inherited
    type VoteType: Default + From<SupportedVoteTypes<Self::Signal>>;

    fn open_share_group_vote(
        organization: u32,
        share_id: u32,
        // uuid generation should default happen when this is called (None is default)
        vote_id: Option<u32>,
        vote_type: Self::VoteType,
        threshold_config: Self::ThresholdConfig,
        duration: Option<BlockNumber>,
    ) -> Result<u32, DispatchError>;
}

/// Define the rate at which signal is minted for shares in an organization
pub trait MintableSignal<AccountId, BlockNumber, FineArithmetic: PerThing>:
    OpenShareGroupVote<AccountId, BlockNumber, FineArithmetic>
{
    fn mint_custom_signal_for_account(
        organization: u32,
        share_id: u32,
        vote_id: u32,
        who: &AccountId,
        signal: Self::Signal,
    );

    fn batch_mint_signal_for_1p1v_share_group(
        organization: u32,
        share_id: u32,
        vote_id: u32,
    ) -> Result<Self::Signal, DispatchError>;

    /// Mints signal for all accounts participating in the vote based on group share allocation from the ShareData module
    fn batch_mint_signal_for_weighted_share_group(
        organization: u32,
        share_id: u32,
        vote_id: u32,
    ) -> Result<Self::Signal, DispatchError>;
}

/// Define the rate at which signal is burned to unreserve shares in an organization
pub trait BurnableSignal<AccountId, BlockNumber, FineArithmetic: PerThing>:
    MintableSignal<AccountId, BlockNumber, FineArithmetic>
{
    fn burn_signal(
        organization: u32,
        share_id: u32,
        vote_id: u32,
        who: &AccountId,
        amount: Option<Self::Signal>,
    ) -> DispatchResult;
}

/// Defines conditions for vote passage (for `VoteState`)
pub trait Approved {
    fn approved(&self) -> bool;
}
pub trait Rejected {
    fn rejected(&self) -> bool;
}
/// Defines how `Vote`s are applied to the `VoteState`
pub trait Apply<Vote>: Sized {
    fn apply(&self, vote: Vote) -> Self;
}
/// Defines how previous `Vote` to the `VoteState` applications are reverted
pub trait Revert<Vote>: Sized {
    fn revert(&self, vote: Vote) -> Self;
}

pub trait VoteVector<Signal, Direction> {
    fn magnitude(&self) -> Signal;
    fn direction(&self) -> Direction;
}

/// Applies vote in the context of the existing module instance
pub trait ApplyVote: GetVoteOutcome {
    type Direction;
    type Vote: VoteVector<Self::Signal, Self::Direction>;
    type State: Approved + Apply<Self::Vote> + Revert<Self::Vote>;

    fn apply_vote(
        state: Self::State,
        new_vote: Self::Vote,
        old_vote: Option<Self::Vote>,
    ) -> Result<Self::State, DispatchError>;
}

/// For the module to check the status of the vote in the context of the existing module instance
pub trait CheckVoteStatus: ApplyVote {
    fn check_vote_outcome(state: Self::State) -> Result<Self::Outcome, DispatchError>;
    fn check_vote_expired(state: Self::State) -> bool;
}

/// For module to update vote state
pub trait VoteOnProposal<AccountId, Hash, BlockNumber, FineArithmetic: PerThing>:
    OpenShareGroupVote<AccountId, BlockNumber, FineArithmetic> + CheckVoteStatus
{
    fn vote_on_proposal(
        organization: u32,
        share_id: u32,
        vote_id: u32,
        voter: AccountId,
        direction: Self::Direction,
        magnitude: Option<Self::Signal>,
        justification: Option<Hash>,
    ) -> DispatchResult;
}

// ====== Vote Dispatch Logic (in Bank) ======

pub trait GetCurrentVoteIdentifiers {
    fn get_current_share_id(&self) -> u32;
    fn get_current_vote_id(&self) -> u32;
}

/// Set the default order of share groups for which approval will be required
/// - the first step to set up a default vote schedule for a proposal type
pub trait SetDefaultShareApprovalOrder {
    type ProposalType;

    fn set_default_share_approval_order_for_proposal_type(
        organization: u32,
        proposal_type: Self::ProposalType,
        share_approval_order: Vec<u32>,
    ) -> DispatchResult;
}

/// Set the default passage, turnout thresholds for each share group
/// - the _second_ first step to set up a default vote schedule for a proposal type
pub trait SetDefaultShareIdThreshold: SetDefaultShareApprovalOrder {
    type ThresholdConfig;

    fn set_share_id_proposal_type_to_threshold(
        organization: u32,
        share_id: u32,
        proposal_type: Self::ProposalType,
        threshold: Self::ThresholdConfig,
    ) -> DispatchResult;
}

/// Helper methods to define a default VoteSchedule using the default threshold setter and default share approval order setter
pub trait VoteScheduleBuilder: SetDefaultShareIdThreshold {
    type ScheduledVote;

    /// Uses the default threshold set above to automatically set threshold for share_id
    fn scheduled_vote_from_share_id_proposal_type(
        organization: u32,
        share_id: u32,
        proposal_type: Self::ProposalType,
        // if None, use default set further above
        custom_threshold: Option<Self::ThresholdConfig>,
    ) -> Result<Self::ScheduledVote, DispatchError>;

    /// Default uses the default share approval order and default threshold setter to set a default vote schedule
    /// - if `raw_vote_schedule.is_some()` then it uses this custom sequence of scheduled votes instead of the defaults
    fn set_default_vote_schedule_for_proposal_type(
        organization: u32,
        proposal_type: Self::ProposalType,
        // if None, use the default share approval order
        raw_vote_schedule: Option<Vec<Self::ScheduledVote>>,
    ) -> DispatchResult;
}

/// Manages live vote schedules
pub trait ManageVoteSchedule: SetDefaultShareApprovalOrder {
    type VoteSchedule: GetCurrentVoteIdentifiers;

    fn dispatch_vote_schedule_from_vec_of_share_id(
        organization: u32,
        proposal_type: Self::ProposalType,
        share_ids: Vec<u32>,
    ) -> Result<Self::VoteSchedule, DispatchError>;

    /// Moves the vote schedule to the next scheduled vote in the sequence
    fn move_to_next_scheduled_vote(
        organization: u32,
        schedule: Self::VoteSchedule,
    ) -> Result<Option<Self::VoteSchedule>, DispatchError>;
}

/// Default uses the default vote schedule configured in `VoteBuilder` to dispatch a `VoteSchedule`
/// - if `custom_share_ids.is_some()` then this is used as the share approval order instead of the default
/// share approval order
pub trait ScheduleVoteSequence: VoteScheduleBuilder {
    // this returns the first `VoteId` and stores the rest in a vote schedule in storage
    fn schedule_default_vote_schedule_for_proposal_type(
        organization: u32,
        proposal_index: u32,
        proposal_type: Self::ProposalType,
        // if None, just use the default vote schedule
        custom_share_ids: Option<Vec<u32>>,
    ) -> Result<u32, DispatchError>; // returns VoteId
}

/// Checks the progress of a scheduled vote sequence and pushes the schedule along
/// - this should be called every `T::PollingFrequency::get()` number of blocks in `on_finalize`
pub trait PollActiveProposal: ScheduleVoteSequence {
    type PollingOutcome;
    // This method checks the outcome of the current vote and moves the schedule to the next one when the threshold is met
    // - returns the newest `VoteId` when the voting schedule is pushed to the next vote
    fn poll_active_proposal(
        organization: u32,
        proposal_index: u32,
    ) -> Result<Self::PollingOutcome, DispatchError>;
}

// ~~~~~~~~ Org Module ~~~~~~~~

// helpers, they are just abstractions over inherited functions
pub trait OrgChecks<OrgId, AccountId> {
    fn check_org_existence(org: OrgId) -> bool;
    fn check_membership_in_org(org: OrgId, account: &AccountId) -> bool;
}

// helpers, they are just abstractions over inherited functions
pub trait ShareGroupChecks<OrgId, AccountId> {
    type MultiShareIdentifier: From<crate::organization::ShareID>; // organization::ShareID
    fn check_share_group_existence(org: OrgId, share_group: Self::MultiShareIdentifier) -> bool;
    fn check_membership_in_share_group(
        org: OrgId,
        share_group: Self::MultiShareIdentifier,
        account: &AccountId,
    ) -> bool;
}

pub trait SupervisorPermissions<OrgId, AccountId>: ShareGroupChecks<OrgId, AccountId> {
    fn is_sudo_account(who: &AccountId) -> bool;
    fn is_organization_supervisor(organization: OrgId, who: &AccountId) -> bool;
    fn is_share_supervisor(
        organization: OrgId,
        share_id: Self::MultiShareIdentifier,
        who: &AccountId,
    ) -> bool;
    // infallible, not protected in any way
    fn put_sudo_account(who: AccountId);
    fn put_organization_supervisor(organization: OrgId, who: AccountId);
    fn put_share_group_supervisor(
        organization: OrgId,
        share_id: Self::MultiShareIdentifier,
        who: AccountId,
    );
    // CAS by default to enforce existing permissions and isolate logic
    fn set_sudo_account(setter: &AccountId, new: AccountId) -> DispatchResult;
    fn set_organization_supervisor(
        organization: OrgId,
        setter: &AccountId,
        new: AccountId,
    ) -> DispatchResult;
    fn set_share_supervisor(
        organization: OrgId,
        share_id: Self::MultiShareIdentifier,
        setter: &AccountId,
        new: AccountId,
    ) -> DispatchResult;
}

// TODO: make `ShareGroupChecks` inherit this && WeightedShareWrapper
pub trait FlatShareWrapper<OrgId, FlatShareId, AccountId> {
    fn get_flat_share_group(
        organization: OrgId,
        share_id: FlatShareId,
    ) -> Result<Vec<AccountId>, DispatchError>;
    fn generate_unique_flat_share_id(organization: OrgId) -> FlatShareId;
    fn add_members_to_flat_share_group(
        organization: OrgId,
        share_id: FlatShareId,
        members: Vec<AccountId>,
    );
}

pub trait WeightedShareWrapper<OrgId, WeightedShareId, AccountId> {
    type Shares: Parameter + Member + AtLeast32Bit + Codec; // exists only to pass inheritance to modules that inherit org
    type Genesis;
    fn get_weighted_shares_for_member(
        organization: OrgId,
        share_id: WeightedShareId,
        member: &AccountId,
    ) -> Result<Self::Shares, DispatchError>;
    fn get_weighted_share_group(
        organization: OrgId,
        share_id: WeightedShareId,
    ) -> Result<Self::Genesis, DispatchError>;
    fn get_outstanding_weighted_shares(
        organization: OrgId,
        share_id: WeightedShareId,
    ) -> Result<Self::Shares, DispatchError>;
    fn generate_unique_weighted_share_id(organization: OrgId) -> WeightedShareId;
}

pub trait WeightedShareIssuanceWrapper<OrgId, WeightedShareId, AccountId, FineArithmetic>:
    WeightedShareWrapper<OrgId, WeightedShareId, AccountId>
{
    fn issue_weighted_shares_from_accounts(
        organization: OrgId,
        members: Vec<(AccountId, Self::Shares)>,
    ) -> Result<WeightedShareId, DispatchError>;
    // TODO: add issue for_member like this
    fn burn_weighted_shares_for_member(
        organization: OrgId,
        share_id: WeightedShareId,
        account: AccountId,
        amount_to_burn: Option<FineArithmetic>, // at some point, replace with portion
    ) -> Result<Self::Shares, DispatchError>;
}

// TODO: FlatShareGroup utilities
pub trait RegisterShareGroup<OrgId, WeightedShareId, AccountId, Shares>:
    ShareGroupChecks<OrgId, AccountId> + WeightedShareWrapper<OrgId, WeightedShareId, AccountId>
{
    fn register_inner_flat_share_group(
        organization: u32,
        group: Vec<AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError>;
    fn register_inner_weighted_share_group(
        organization: u32,
        group: Vec<(AccountId, Shares)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError>;
    fn register_outer_flat_share_group(
        organization: u32,
        group: Vec<AccountId>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError>;
    fn register_outer_weighted_share_group(
        organization: u32,
        group: Vec<(AccountId, Shares)>,
    ) -> Result<Self::MultiShareIdentifier, DispatchError>;
}

pub trait GetInnerOuterShareGroups<OrgId, AccountId>: ShareGroupChecks<OrgId, AccountId> {
    fn get_inner_share_group_identifiers(
        organization: OrgId,
    ) -> Option<Vec<Self::MultiShareIdentifier>>;
    fn get_outer_share_group_identifiers(
        organization: OrgId,
    ) -> Option<Vec<Self::MultiShareIdentifier>>;
}

pub trait OrganizationDNS<OrgId, AccountId, Hash>: OrgChecks<OrgId, AccountId> {
    type OrgSrc;
    type OrganizationState;
    // called to form the organization in the method below
    fn organization_from_src(
        src: Self::OrgSrc,
        org_id: OrgId,
        value_constitution: Hash,
    ) -> Result<Self::OrganizationState, DispatchError>;
    fn register_organization(
        source: Self::OrgSrc,
        value_constitution: Hash,
        supervisor: Option<AccountId>,
    ) -> Result<(OrgId, Self::OrganizationState), DispatchError>; // returns OrgId in this module's context
}

// ~~~~~~~~ Bank Module ~~~~~~~~

pub trait SupportedOrganizationShapes {
    type FormedOrgId; // see crate::organization::FormedOrganization
}

pub trait RegisterOffChainBankAccount: SupportedOrganizationShapes {
    type TreasuryId;
    fn register_off_chain_bank_account(
        org: Self::FormedOrgId,
    ) -> Result<Self::TreasuryId, DispatchError>;
}

pub trait OffChainBank: RegisterOffChainBankAccount {
    type Payment;

    fn sender_claims_payment_sent(id: Self::TreasuryId, payment: Self::Payment) -> Self::Payment;
    fn recipient_confirms_payment_received(
        id: Self::TreasuryId,
        payment: Self::Payment,
    ) -> DispatchResult;
    fn check_payment_confirmation(id: Self::TreasuryId, payment: Self::Payment) -> bool;
}

pub trait RegisterOnChainBankAccount<AccountId, Currency, FineArithmetic> {
    type TreasuryId: Clone;
    type WithdrawRules;
    fn register_on_chain_bank_account(
        from: AccountId,
        amount: Currency,
        pct_reserved_for_spends: Option<FineArithmetic>,
        permissions: Self::WithdrawRules,
    ) -> Result<Self::TreasuryId, DispatchError>;
}

pub trait VerifyOwnership<OrgShape>: Sized {
    fn verify_ownership(&self, org: OrgShape) -> bool;
}

pub trait GetBalance<Currency>: Sized {
    fn get_savings(&self) -> Currency;
    fn get_reserved_for_spends(&self) -> Currency;
    fn get_total_balance(&self) -> Currency;
}

pub trait DepositWithdrawalOps<Currency, FineArithmetic>: Sized {
    // infallible
    fn apply_deposit(&self, amount: Currency, pct_savings: Option<FineArithmetic>) -> Self;
    // fallible, not enough capital
    fn spend_from_total(&self, amount: Currency) -> Option<Self>;
    fn spend_from_reserved_spends(&self, amt: Currency) -> Option<Self>;
    fn spend_from_savings(&self, amt: Currency) -> Option<Self>;
}

pub trait ChangeBankBalances<Currency, FineArithmetic>: SupportedOrganizationShapes {
    type Bank: DepositWithdrawalOps<Currency, FineArithmetic>
        + VerifyOwnership<Self::FormedOrgId>
        + GetBalance<Currency>;
    fn make_deposit_to_update_bank_balance(
        bank: Self::Bank,
        amount: Currency,
        pct_savings: Option<FineArithmetic>,
    ) -> Self::Bank;
    fn request_withdrawal_to_update_bank_balance(
        bank: Self::Bank,
        amount: Currency,
        savings: bool,             // true if these funds are available to callee
        reserved_for_spends: bool, // true if these funds are available to callee
    ) -> Result<Self::Bank, DispatchError>;
}

pub trait CheckBankBalances<AccountId, Currency, FineArithmetic>:
    RegisterOnChainBankAccount<AccountId, Currency, FineArithmetic>
    + ChangeBankBalances<Currency, FineArithmetic>
{
    fn get_bank(bank_id: Self::TreasuryId) -> Option<Self::Bank>;
    fn get_bank_total_balance(bank_id: Self::TreasuryId) -> Option<Currency>;
}

pub trait OnChainBank<AccountId, Hash, Currency, FineArithmetic>:
    RegisterOnChainBankAccount<AccountId, Currency, FineArithmetic>
{
    fn deposit_currency_into_on_chain_bank_account(
        from: AccountId,
        to_bank_id: Self::TreasuryId,
        amount: Currency,
        savings_tax: Option<FineArithmetic>,
        reason: Hash,
    ) -> DispatchResult;
    // NEVER TO BE CALLED DIRECTLY, must be wrapped in some other API
    fn withdraw_from_on_chain_bank_account(
        from_bank_id: Self::TreasuryId,
        to: AccountId,
        amount: Currency,
        savings: bool,             // true if these funds are available to callee
        reserved_for_spends: bool, // true if these funds are available to callee
    ) -> DispatchResult;
}

pub trait GetDepositsByAccountForBank<AccountId, Hash, Currency, FineArithmetic>:
    OnChainBank<AccountId, Hash, Currency, FineArithmetic>
{
    type DepositInfo;
    fn get_deposits_by_account(
        bank_id: Self::TreasuryId,
        depositer: AccountId,
    ) -> Option<Vec<Self::DepositInfo>>;
    fn total_capital_deposited_by_account(
        bank_id: Self::TreasuryId,
        depositer: AccountId,
    ) -> Currency;
}
// all operations are done with calculations done at the time the request is processed
// - this leads to some problems because requests automatically execute at values that change
pub trait OnChainWithdrawalFilters<AccountId, Hash, Currency, FineArithmetic>:
    GetDepositsByAccountForBank<AccountId, Hash, Currency, FineArithmetic>
{
    // no guarantees on the value this returns, on chain conditions change fast
    fn calculate_liquid_portion_of_on_chain_deposit(
        from_bank_id: Self::TreasuryId,
        deposit: Self::DepositInfo,
        to_claimer: AccountId,
    ) -> Result<Currency, DispatchError>;
    // no guarantees on the value this returns, on chain conditions change fast
    fn calculate_liquid_share_capital_from_savings(
        from_bank_id: Self::TreasuryId,
        to_claimer: AccountId,
    ) -> Result<(u32, u32, Currency), DispatchError>;
    // request for a portion of an on-chain deposit, the impl defines what determines the fair portion
    fn claim_portion_of_on_chain_deposit(
        from_bank_id: Self::TreasuryId,
        deposit: Self::DepositInfo,
        to_claimer: AccountId,
        amount: Option<Currency>,
    ) -> Result<Currency, DispatchError>;
    // irreversible decision to sell ownership in exchange for a portion of the collateral
    // - automatically calculated according to the proportion of ownership at the time the request is processed
    // -- NOTE: this does not shield against dilution if there is a run on the collateral because it does not yield a limit order for the share sale
    fn withdraw_capital_by_burning_shares(
        from_bank_id: Self::TreasuryId,
        to_claimer: AccountId,
        amount: Option<Currency>, // if None, as much as possible
    ) -> Result<Currency, DispatchError>;
}

// ~~~~~~~~ Bounty Module ~~~~~~~~

pub trait CreateBounty<IpfsReference, Currency>: SupportedOrganizationShapes {
    type BankId: Clone;
    type ReviewCommittee;
    // helper to screen, prepare and form bounty information object
    fn screen_bounty_submission(
        caller: Self::FormedOrgId,
        description: IpfsReference,
        bank_account: Self::BankId,
        amount_reserved_for_bounty: Currency, // collateral requirement
        amount_claimed_available: Currency,   // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> DispatchResult;
    // call should be an authenticated member of the FormedOrgId
    // - could be the inner shares of an organization for example
    fn create_bounty(
        caller: Self::FormedOrgId,
        description: IpfsReference,
        bank_account: Self::BankId,
        amount_reserved_for_bounty: Currency, // collateral requirement
        amount_claimed_available: Currency,   // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<(Self::FormedOrgId, u32), DispatchError>;
}
