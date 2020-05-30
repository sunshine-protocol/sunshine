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

pub trait GenerateUniqueID<Id> {
    fn generate_unique_id() -> Id;
}

pub trait SeededGenerateUniqueID<Id, Seed> {
    fn seeded_generate_unique_id(seed: Seed) -> Id;
}

pub trait GenerateUniqueKeyID<KeyId> {
    fn generate_unique_key_id(proposed: KeyId) -> KeyId;
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
// --
// GetTotalShareIssuance is in WeightedShareGroup::outstanding_shares
// --
// pub trait GetWeightedShareGroupShape<AccountId, Shares>: GetTotalShareIssuance<Shares> {
//     fn get_weighted_share_group_shape(
//         organization: u32,
//         share_id: u32,
//     ) -> Result<Vec<(AccountId, Shares)>, DispatchError>;
// }

// ---------- Petition Logic ----------

// impl GetVoteOutcome

pub trait OpenPetition<Hash, BlockNumber>: GetVoteOutcome {
    fn open_petition(
        organization: u32,
        share_id: u32,
        topic: Option<Hash>,
        required_support: u32,
        require_against: Option<u32>,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteId, DispatchError>;
    // why do we need this? because we only have context for total_electorate in this method,
    // not outside of it so we can't just pass total_electorate into `open_petition`
    fn open_unanimous_approval_petition(
        organization: u32,
        share_id: u32,
        topic: Option<Hash>,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteId, DispatchError>;
}

pub trait UpdatePetitionTerms<Hash>: Sized {
    fn update_petition_terms(&self, new_terms: Hash) -> Self;
}

pub trait SignPetition<AccountId, Hash>: GetVoteOutcome {
    type Petition: Approved + Rejected + UpdatePetitionTerms<Hash> + Apply<Self::SignerView>;
    type SignerView;
    fn check_petition_outcome(petition: Self::Petition) -> Result<Self::Outcome, DispatchError>;
    fn sign_petition(
        petition_id: Self::VoteId,
        signer: AccountId,
        view: Self::SignerView,
    ) -> Result<Self::Outcome, DispatchError>;
}

// get full veto context by filtering on the `SignatureLogger` map O(n)

pub trait RequestChanges<AccountId, Hash>: SignPetition<AccountId, Hash> {
    fn request_changes(
        petition_id: Self::VoteId,
        signer: AccountId,
        justification: Hash,
    ) -> Result<Option<Self::Outcome>, DispatchError>;
    fn accept_changes(
        petition_id: Self::VoteId,
        signer: AccountId,
    ) -> Result<Option<Self::Outcome>, DispatchError>;
}

pub trait UpdatePetition<AccountId, Hash>: SignPetition<AccountId, Hash> {
    fn update_petition(petition_id: u32, new_topic: Hash) -> DispatchResult;
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
pub trait AccessProfile<Shares> {
    fn total(&self) -> Shares;
}

pub trait WeightedShareGroup<AccountId> {
    type Shares: Parameter + Member + AtLeast32Bit + Codec;
    type Profile: AccessProfile<Self::Shares>;
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
    ) -> Option<Self::Profile>;
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
    type VoteId: Into<u32> + From<u32>; // Into<u32> is necessary to make it work
    type Outcome: Approved;

    fn get_vote_outcome(vote_id: Self::VoteId) -> Result<Self::Outcome, DispatchError>;
}

pub trait ThresholdVote {
    type Signal: Parameter + Member + AtLeast32Bit + Codec;
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
    GetVoteOutcome + ThresholdVote
{
    type ThresholdConfig: DeriveThresholdRequirement<Self::Signal>
        + ConsistentThresholdStructure
        + From<ThresholdConfig<Self::Signal, FineArithmetic>>
        + From<ThresholdConfigBuilder<FineArithmetic>>; // NOTE: this forces FineArithmetic generic parameter for traits and all inherited
    type VoteType: Default + From<SupportedVoteTypes>;

    fn open_share_group_vote(
        organization: u32,
        share_id: u32,
        vote_type: Self::VoteType,
        threshold_config: Self::ThresholdConfig,
        duration: Option<BlockNumber>,
    ) -> Result<Self::VoteId, DispatchError>;
}

/// Define the rate at which signal is minted for shares in an organization
pub trait MintableSignal<AccountId, BlockNumber, FineArithmetic: PerThing>:
    OpenShareGroupVote<AccountId, BlockNumber, FineArithmetic>
{
    fn mint_custom_signal_for_account(vote_id: u32, who: &AccountId, signal: Self::Signal);

    fn batch_mint_signal_for_1p1v_share_group(
        organization: u32,
        share_id: u32,
    ) -> Result<(Self::VoteId, Self::Signal), DispatchError>;

    /// Mints signal for all accounts participating in the vote based on group share allocation from the ShareData module
    fn batch_mint_signal_for_weighted_share_group(
        organization: u32,
        share_id: u32,
    ) -> Result<(Self::VoteId, Self::Signal), DispatchError>;
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

pub trait UpdateOutcome: Sized {
    // only returns if outcome changes
    fn update_outcome(&self) -> Option<Self>;
}

pub trait VoteVector<Signal, Direction> {
    fn magnitude(&self) -> Signal;
    fn direction(&self) -> Direction;
}

/// Applies vote in the context of the existing module instance
pub trait ApplyVote: GetVoteOutcome + ThresholdVote {
    type Direction;
    type Vote: VoteVector<Self::Signal, Self::Direction>;
    type State: Approved + Apply<Self::Vote> + Revert<Self::Vote> + UpdateOutcome;
    // apply vote to vote state
    fn apply_vote(
        state: Self::State,
        new_vote: Self::Vote,
        old_vote: Option<Self::Vote>,
    ) -> Result<(Self::State, Option<(bool, Self::Signal)>), DispatchError>;
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
    fn get_org_size(org: OrgId) -> u32;
}

// helpers, they are just abstractions over inherited functions
pub trait ShareGroupChecks<OrgId, ShareId, AccountId> {
    fn check_share_group_existence(org: OrgId, share_group: ShareId) -> bool;
    fn check_membership_in_share_group(
        org: OrgId,
        share_group: ShareId,
        account: &AccountId,
    ) -> bool;
    fn get_share_group_size(org: OrgId, share_group: ShareId) -> u32;
}

pub trait SupervisorPermissions<OrgId, ShareId, AccountId>:
    ShareGroupChecks<OrgId, ShareId, AccountId>
{
    fn is_sudo_account(who: &AccountId) -> bool;
    fn is_organization_supervisor(organization: OrgId, who: &AccountId) -> bool;
    fn is_share_supervisor(organization: OrgId, share_id: ShareId, who: &AccountId) -> bool;
    // infallible, not protected in any way
    fn put_sudo_account(who: AccountId);
    fn put_organization_supervisor(organization: OrgId, who: AccountId);
    fn put_share_group_supervisor(organization: OrgId, share_id: ShareId, who: AccountId);
    // CAS by default to enforce existing permissions and isolate logic
    fn set_sudo_account(setter: &AccountId, new: AccountId) -> DispatchResult;
    fn set_organization_supervisor(
        organization: OrgId,
        setter: &AccountId,
        new: AccountId,
    ) -> DispatchResult;
    fn set_share_supervisor(
        organization: OrgId,
        share_id: ShareId,
        setter: &AccountId,
        new: AccountId,
    ) -> DispatchResult;
}

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
    type Profile: AccessProfile<Self::Shares>;
    type Genesis;
    fn get_member_share_profile(
        organization: OrgId,
        share_id: WeightedShareId,
        member: &AccountId,
    ) -> Option<Self::Profile>;
    fn get_weighted_share_group(
        organization: OrgId,
        share_id: WeightedShareId,
    ) -> Result<Self::Genesis, DispatchError>;
    fn get_outstanding_weighted_shares(
        organization: OrgId,
        share_id: WeightedShareId,
    ) -> Option<Self::Shares>;
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

pub trait RegisterShareGroup<OrgId, ShareId, AccountId, Shares>:
    ShareGroupChecks<OrgId, ShareId, AccountId>
{
    fn register_inner_flat_share_group(
        organization: OrgId,
        group: Vec<AccountId>,
    ) -> Result<ShareId, DispatchError>;
    fn register_inner_weighted_share_group(
        organization: OrgId,
        group: Vec<(AccountId, Shares)>,
    ) -> Result<ShareId, DispatchError>;
    fn register_outer_flat_share_group(
        organization: u32,
        group: Vec<AccountId>,
    ) -> Result<ShareId, DispatchError>;
    fn register_outer_weighted_share_group(
        organization: u32,
        group: Vec<(AccountId, Shares)>,
    ) -> Result<ShareId, DispatchError>;
}

pub trait GetInnerOuterShareGroups<OrgId, ShareId, AccountId>:
    ShareGroupChecks<OrgId, ShareId, AccountId>
{
    fn get_inner_share_group_identifiers(organization: OrgId) -> Option<Vec<ShareId>>;
    fn get_outer_share_group_identifiers(organization: OrgId) -> Option<Vec<ShareId>>;
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

// ~~~~~~~~ BankOffChain Module ~~~~~~~~

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

// ~~~~~~~~ BankOnChain Module ~~~~~~~~
use crate::bank::OnChainTreasuryID;
pub trait OnChainBank {
    type OrgId: From<u32>;
    type TreasuryId: Clone + From<OnChainTreasuryID>;
}
pub trait RegisterBankAccount<AccountId, GovernanceConfig, Currency>: OnChainBank {
    // requires a deposit of some size above the minimum and returns the OnChainTreasuryID
    fn register_on_chain_bank_account(
        registered_org: Self::OrgId,
        from: AccountId,
        amount: Currency,
        owner_s: GovernanceConfig,
    ) -> Result<Self::TreasuryId, DispatchError>;
    fn check_bank_owner(bank_id: Self::TreasuryId, org: Self::OrgId) -> bool;
} // people should be eventually able to solicit loans from others to SEED a bank account but they cede some or all of the control...

pub trait OwnershipProportionCalculations<AccountId, GovernanceConfig, Currency, FineArithmetic>:
    RegisterBankAccount<AccountId, GovernanceConfig, Currency>
{
    fn calculate_proportion_ownership_for_account(
        account: AccountId,
        group: GovernanceConfig,
    ) -> Option<FineArithmetic>;
    fn calculate_proportional_amount_for_account(
        amount: Currency,
        account: AccountId,
        group: GovernanceConfig,
    ) -> Option<Currency>;
}

pub trait FreeToReserved<Currency>: Sized {
    // fallible, requires enough in `free`
    fn move_from_free_to_reserved(&self, amount: Currency) -> Option<Self>;
}

pub trait GetBalance<Currency>: Sized {
    fn total_free_funds(&self) -> Currency;
    fn total_reserved_funds(&self) -> Currency;
    fn total_funds(&self) -> Currency;
}

pub trait DepositSpendOps<Currency>: Sized {
    // infallible
    fn deposit_into_free(&self, amount: Currency) -> Self;
    fn deposit_into_reserved(&self, amount: Currency) -> Self;
    // fallible, not enough capital in relative account
    fn spend_from_free(&self, amount: Currency) -> Option<Self>;
    fn spend_from_reserved(&self, amount: Currency) -> Option<Self>;
}

// notably, !\exists deposit_into_reservation || spend_from_free because those aren't supported _here_
pub trait BankDepositsAndSpends<Currency> {
    type Bank: DepositSpendOps<Currency> + GetBalance<Currency> + FreeToReserved<Currency>;
    fn make_infallible_deposit_into_free(bank: Self::Bank, amount: Currency) -> Self::Bank;
    // returns option if the `DepositSpendOps` does, propagate that NotEnoughFundsError
    fn fallible_spend_from_reserved(
        bank: Self::Bank,
        amount: Currency,
    ) -> Result<Self::Bank, DispatchError>;
    fn fallible_spend_from_free(
        bank: Self::Bank,
        amount: Currency,
    ) -> Result<Self::Bank, DispatchError>;
}

// useful for testing, the invariant is that the storage item returned from the first method should have self.free + self.reserved == the balance returned from the second method (for the same bank_id)
pub trait CheckBankBalances<Currency>: OnChainBank + BankDepositsAndSpends<Currency> {
    // prefer this method in most cases because
    fn get_bank_store(bank_id: Self::TreasuryId) -> Option<Self::Bank>;
    // -> invariant for module is that this returns the same as if you calculate total balance from the above storage item
    fn calculate_total_bank_balance_from_balances(bank_id: Self::TreasuryId) -> Option<Currency>;
}

pub trait DepositIntoBank<AccountId, GovernanceConfig, Hash, Currency>:
    RegisterBankAccount<AccountId, GovernanceConfig, Currency> + BankDepositsAndSpends<Currency>
{
    // get the bank corresponding to bank_id call infallible deposit
    // - only fails if `from` doesn't have enough Currency
    fn deposit_into_bank(
        from: AccountId,
        to_bank_id: Self::TreasuryId,
        amount: Currency,
        reason: Hash,
    ) -> Result<u32, DispatchError>; // returns DepositId
}

// One good question here might be, why are we passing the caller into this
// method and doing authentication in this method instead of doing it in the
// runtime method and just limiting where this is called to places where
// authenticaton occurs before it. The answer is that we're using objects in
// runtime storage to authenticate the call so we need to pass the caller
// into the method -- if we don't do this, we'll require two storage calls
// instead of one because we'll authenticate outside of this method by getting
// the storage item in the runtime method to check auth but then we'll also
// get the storage item in this method (because we don't pass it in and I
// struggle to see a clean design in which we pass it in but don't
// encourage/enable unsafe puts)
pub trait BankReservations<AccountId, GovernanceConfig, Currency, Hash>:
    RegisterBankAccount<AccountId, GovernanceConfig, Currency>
{
    fn reserve_for_spend(
        caller: AccountId, // must be in owner_s: GovernanceConfig for BankState, that's the auth
        bank_id: Self::TreasuryId,
        reason: Hash,
        amount: Currency,
        // acceptance committee for approving set aside spends below the amount
        controller: GovernanceConfig,
    ) -> Result<u32, DispatchError>;
    // only reserve.controller() can unreserve funds after commitment (with method further down)
    fn commit_reserved_spend_for_transfer(
        caller: AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        reason: Hash,
        amount: Currency,
        expected_future_owner: GovernanceConfig,
    ) -> DispatchResult;
    // bank controller can unreserve if not committed
    fn unreserve_uncommitted_to_make_free(
        caller: AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        amount: Currency,
    ) -> DispatchResult;
    // reservation.controller() can unreserve committed funds
    fn unreserve_committed_to_make_free(
        caller: AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        amount: Currency,
    ) -> DispatchResult;
    // reservation.controller() transfers control power to new_controller and enables liquidity by this controller
    fn transfer_spending_power(
        caller: AccountId,
        bank_id: Self::TreasuryId,
        reason: Hash,
        // reference to specific reservation
        reservation_id: u32,
        amount: Currency,
        // move control of funds to new outer group which can reserve or withdraw directly
        new_controller: GovernanceConfig,
    ) -> DispatchResult;
}

pub trait BankSpends<AccountId, GovernanceConfig, Currency>:
    OnChainBank + RegisterBankAccount<AccountId, GovernanceConfig, Currency>
{
    fn spend_from_free(
        from_bank_id: Self::TreasuryId,
        to: AccountId,
        amount: Currency,
    ) -> DispatchResult;
    fn spend_from_transfers(
        from_bank_id: Self::TreasuryId,
        // transfer_id
        id: u32,
        to: AccountId,
        amount: Currency,
    ) -> Result<Currency, DispatchError>;
}

// soon-to-be new module: Term Sheet

// Note to Self: the game theoretic move will be to unreserve all the capital and trade it
// so that has to be controlled in the context of this request. There are a few options to solve
// (1)  require a significant enough delay between unreserving and calling this
// (2) rate limit the number of `reservations` and `unreservations` for each member
// (3) if liquidating, automatically exercise rate limit unreserve for reserved, uncommitted capital
// pub trait TradeOwnershipForFreeCapital

// ~ in bank now for demo purposes, this is mvp rage_quit
pub trait TermSheetExit<AccountId, Currency>: OnChainBank {
    fn burn_shares_to_exit_bank_ownership(
        rage_quitter: AccountId,
        bank_id: Self::TreasuryId,
    ) -> Result<Currency, DispatchError>;
} // TODO: method to trade some ownership for some free capital instead of making ownership atomic, but it should be atomic for the simplest version

pub trait TermSheetIssuance<AccountId, Hash, Shares, Currency>: OnChainBank {
    type VoteConfig; // enum to express supported vote options

    // apply to DAO
    fn apply_for_bank_ownership(
        bank_id: Self::TreasuryId,
        applicant: AccountId,
        stake_promised: Currency,
        shares_requested: Shares,
        application: Hash,
    ) -> Result<u32, DispatchError>; // returns Ok(ApplicationId)

    // sponsor application to trigger vote (only requires one member)
    fn sponsor_application_to_trigger_vote(
        caller: AccountId,
        bank_id: Self::TreasuryId,
        application_id: u32,
        stake_promised: Currency,
        shares_requested: Shares,
        application: Hash,
    ) -> Result<u32, DispatchError>; // returns Ok(VoteId)

    // polling method to check the vote module and make changes in this module if necessary for issuance
    // -> requires an application's relevant vote to be approved
    fn poll_vote_result_to_enforce_outcome(
        bank_id: Self::TreasuryId,
        vote_id: u32,
    ) -> DispatchResult;
}

pub trait CommitSpendReservation<Currency>: Sized {
    fn commit_spend_reservation(&self, amount: Currency) -> Option<Self>;
}

// primarily useful for unreserving funds to move them back to free
pub trait MoveFundsOutUnCommittedOnly<Currency>: Sized {
    fn move_funds_out_uncommitted_only(&self, amount: Currency) -> Option<Self>;
}

// useful for (1) moving out of spend_reservation to internal transfer
//            (2) moving out of transfer during withdrawal
pub trait MoveFundsOutCommittedOnly<Currency>: Sized {
    fn move_funds_out_committed_only(&self, amount: Currency) -> Option<Self>;
}

pub trait BankStorageInfo<AccountId, GovernanceConfig, Currency>:
    RegisterBankAccount<AccountId, GovernanceConfig, Currency>
{
    type DepositInfo;
    type ReservationInfo: MoveFundsOutUnCommittedOnly<Currency>
        + MoveFundsOutCommittedOnly<Currency>;
    type TransferInfo: MoveFundsOutCommittedOnly<Currency>;
    // deposit
    fn get_deposits_by_account(
        bank_id: Self::TreasuryId,
        depositer: AccountId,
    ) -> Option<Vec<Self::DepositInfo>>;
    fn total_capital_deposited_by_account(
        bank_id: Self::TreasuryId,
        depositer: AccountId,
    ) -> Currency;
    // reservations
    fn get_amount_left_in_spend_reservation(
        bank_id: Self::TreasuryId,
        reservation_id: u32,
    ) -> Option<Currency>;
    fn get_reservations_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: GovernanceConfig,
    ) -> Option<Vec<Self::ReservationInfo>>;
    fn total_capital_reserved_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: GovernanceConfig,
    ) -> Currency;
    // transfers
    fn get_amount_left_in_approved_transfer(
        bank_id: Self::TreasuryId,
        transfer_id: u32,
    ) -> Option<Currency>;
    fn get_transfers_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: GovernanceConfig,
    ) -> Option<Vec<Self::TransferInfo>>;
    fn total_capital_transferred_to_governance_config(
        bank_id: Self::TreasuryId,
        invoker: GovernanceConfig,
    ) -> Currency;
}

// ~~~~~~~~ Bounty Module ~~~~~~~~

pub trait FoundationParts {
    type OrgId;
    type BountyId;
    type BankId;
    type MultiShareId;
    type MultiVoteId;
    type TeamId;
}

// TODO: this could be removed if we didn't cache the ownership of on-chain
// banks in bounty and instead checked ownership in the `screen_bounty_creation` `reserve_spend` call
// to bank but I don't think it's the worst thing to have for V1
pub trait RegisterFoundation<Currency, AccountId>: FoundationParts {
    // should still be some minimum enforced in bank
    fn register_foundation_from_donation_deposit(
        from: AccountId,
        for_org: Self::OrgId,
        amount: Currency,
    ) -> Result<Self::BankId, DispatchError>;
    fn register_foundation_from_existing_bank(
        org: Self::OrgId,
        bank: Self::BankId,
    ) -> DispatchResult;
}

pub trait CreateBounty<Currency, AccountId, Hash>: RegisterFoundation<Currency, AccountId> {
    type BountyInfo;
    type ReviewCommittee;
    // helper to screen, prepare and form bounty information object
    fn screen_bounty_creation(
        foundation: Self::OrgId, // registered OrgId
        caller: AccountId,
        bank_account: Self::BankId,
        description: Hash,
        amount_reserved_for_bounty: Currency, // collateral requirement
        amount_claimed_available: Currency,   // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<Self::BountyInfo, DispatchError>;
    // call should be an authenticated member of the OrgId
    // - requirement might be the inner shares of an organization for example
    fn create_bounty(
        foundation: Self::OrgId, // registered OrgId
        caller: AccountId,
        bank_account: Self::BankId,
        description: Hash,
        amount_reserved_for_bounty: Currency, // collateral requirement
        amount_claimed_available: Currency,   // claimed available amount, not necessarily liquid
        acceptance_committee: Self::ReviewCommittee,
        supervision_committee: Option<Self::ReviewCommittee>,
    ) -> Result<Self::BountyId, DispatchError>;
}

pub trait UseTermsOfAgreement<AccountId>: FoundationParts {
    type TermsOfAgreement;
    fn request_consent_on_terms_of_agreement(
        bounty_org: u32,
        terms: Self::TermsOfAgreement,
    ) -> Result<(Self::MultiShareId, Self::MultiVoteId), DispatchError>;
    fn approve_grant_to_register_team(
        bounty_org: u32,
        flat_share_id: u32,
        terms: Self::TermsOfAgreement,
    ) -> Result<Self::TeamId, DispatchError>;
}

pub trait StartApplicationReviewPetition<VoteID> {
    fn start_application_review_petition(&self, vote_id: VoteID) -> Self;
    fn get_application_review_id(&self) -> Option<VoteID>;
}

pub trait StartTeamConsentPetition<ShareID, VoteID> {
    fn start_team_consent_petition(&self, share_id: ShareID, vote_id: VoteID) -> Self;
    fn get_team_consent_id(&self) -> Option<VoteID>;
    fn get_team_flat_id(&self) -> Option<ShareID>;
}

// TODO: clean up the outer_flat_share_id dispatched for team consent if NOT formally approved
pub trait ApproveGrant<TeamID> {
    fn approve_grant(&self, team_id: TeamID) -> Self;
    fn get_full_team_id(&self) -> Option<TeamID>;
}
// TODO: RevokeApprovedGrant<VoteID> => vote to take away the team's grant and clean storage

pub trait SubmitGrantApplication<Currency, AccountId, Hash>:
    CreateBounty<Currency, AccountId, Hash> + UseTermsOfAgreement<AccountId>
{
    type GrantApp: StartApplicationReviewPetition<Self::MultiVoteId>
        + StartTeamConsentPetition<Self::MultiShareId, Self::MultiVoteId>
        + ApproveGrant<Self::TeamId>;
    fn form_grant_application(
        bounty_id: u32,
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: Self::TermsOfAgreement,
    ) -> Result<Self::GrantApp, DispatchError>;
    fn submit_grant_application(
        bounty_id: u32,
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: Self::TermsOfAgreement,
    ) -> Result<u32, DispatchError>; // returns application identifier
}

pub trait SuperviseGrantApplication<Currency, AccountId, Hash>:
    CreateBounty<Currency, AccountId, Hash> + UseTermsOfAgreement<AccountId>
{
    type AppState;
    fn trigger_application_review(
        trigger: AccountId, // must be authorized to trigger in context of objects
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError>;
    // someone can try to call this and only the sudo can push things through at whim
    // -> notably no sudo deny for demo functionality
    fn sudo_approve_application(
        sudo: AccountId,
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError>;
    // this returns the AppState but also pushes it along if necessary
    // - it should be called in on_finalize periodically
    fn poll_application(
        bounty_id: u32,
        application_id: u32,
    ) -> Result<Self::AppState, DispatchError>;
}

pub trait SubmitMilestone<Currency, AccountId, Hash>:
    SuperviseGrantApplication<Currency, AccountId, Hash>
{
    type MilestoneState;
    fn submit_milestone(
        caller: AccountId, // must be from the team, maybe check sudo || flat_org_member
        bounty_id: u32,
        application_id: u32,
        team_id: Self::TeamId,
        submission_reference: Hash,
        amount_requested: Currency,
    ) -> Result<u32, DispatchError>; // returns milestone_id
    fn trigger_milestone_review(
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError>;
    // someone can try to call this and only the sudo can push things through at whim
    fn sudo_approves_milestone(
        caller: AccountId,
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError>;
    fn poll_milestone(
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError>;
}
