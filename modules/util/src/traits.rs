use sp_runtime::{DispatchError, DispatchResult};
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
    // removes any existing sudo and places None
    fn clear_organization_supervisor(org: OrgId) -> DispatchResult;
    // removes any existing sudo and places `who`
    fn put_organization_supervisor(org: OrgId, who: AccountId) -> DispatchResult;
}

// ---------- Membership Logic ----------
pub trait GetGroupSize<OrgId> {
    fn get_size_of_group(org_id: OrgId) -> u32;
}

/// Checks that the `AccountId` is a member of a share group in an organization
pub trait GroupMembership<OrgId, AccountId>: GetGroupSize<OrgId> {
    fn is_member_of_group(org_id: OrgId, who: &AccountId) -> bool;
}

/// All changes to the organizational membership are infallible
pub trait ChangeGroupMembership<OrgId, AccountId>: GroupMembership<OrgId, AccountId> {
    fn add_member_to_org(org_id: OrgId, new_member: AccountId, batch: bool) -> DispatchResult;
    fn remove_member_from_org(org_id: OrgId, old_member: AccountId, batch: bool) -> DispatchResult;
    /// WARNING: the vector fed as inputs to the following methods must have NO duplicates
    fn batch_add_members_to_org(org_id: OrgId, new_members: Vec<AccountId>) -> DispatchResult;
    fn batch_remove_members_from_org(org_id: OrgId, old_members: Vec<AccountId>) -> DispatchResult;
}
pub trait GetGroup<OrgId, AccountId> {
    fn get_group(organization: OrgId) -> Option<Vec<AccountId>>;
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
    fn get_share_profile(organization: OrgId, who: &AccountId) -> Option<Self::Profile>;
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
    fn batch_issue(organization: OrgId, genesis: Self::Genesis) -> DispatchResult;
    fn batch_burn(organization: OrgId, genesis: Self::Genesis) -> DispatchResult;
}
pub trait ReserveProfile<OrgId, AccountId, Shares>:
    ShareIssuance<OrgId, AccountId, Shares>
{
    fn reserve(
        organization: OrgId,
        who: &AccountId,
        amount: Option<Shares>,
    ) -> Result<Shares, DispatchError>;
    fn unreserve(
        organization: OrgId,
        who: &AccountId,
        amount: Option<Shares>,
    ) -> Result<Shares, DispatchError>;
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
    ) -> Result<Self::OrganizationState, DispatchError>;
    fn register_organization(
        source: Self::OrgSrc,
        supervisor: Option<AccountId>,
        value_constitution: Hash,
    ) -> Result<OrgId, DispatchError>; // returns OrgId in this module's context
    fn register_sub_organization(
        parent_id: OrgId,
        source: Self::OrgSrc,
        supervisor: Option<AccountId>,
        value_constitution: Hash,
    ) -> Result<OrgId, DispatchError>;
}
pub trait RemoveOrganization<OrgId> {
    // returns Ok(Some(child_id)) or Ok(None) if leaf org
    fn remove_organization(id: OrgId) -> Result<Option<Vec<OrgId>>, DispatchError>;
    fn recursive_remove_organization(id: OrgId) -> DispatchResult;
}

// ---------- Petition Logic ----------

/// Retrieves the outcome of a vote associated with the vote identifier `vote_id`
pub trait GetVoteOutcome<VoteId> {
    type Outcome;

    fn get_vote_outcome(vote_id: VoteId) -> Result<Self::Outcome, DispatchError>;
}

pub trait OpenPetition<VoteId, OrgId, Hash, BlockNumber>: GetVoteOutcome<VoteId> {
    fn open_petition(
        organization: OrgId,
        topic: Option<Hash>,
        required_support: u32,
        vetos_to_reject: u32,
        duration: Option<BlockNumber>,
    ) -> Result<VoteId, DispatchError>;
    fn open_unanimous_approval_petition(
        organization: OrgId,
        topic: Option<Hash>,
        vetos_to_reject: u32,
        duration: Option<BlockNumber>,
    ) -> Result<VoteId, DispatchError>;
}

pub trait UpdatePetitionTerms<Hash>: Sized {
    fn update_petition_terms(&self, new_terms: Hash, clear_votes_on_update: bool) -> Self;
}

pub trait SignPetition<VoteId, AccountId, Hash, SignerView>: GetVoteOutcome<VoteId> {
    type Petition: Approved + Rejected + UpdatePetitionTerms<Hash> + Apply<SignerView>;
    fn check_petition_outcome(petition_state: Self::Petition) -> Option<Self::Outcome>;
    fn sign_petition(
        petition_id: VoteId,
        signer: AccountId,
        view: SignerView,
    ) -> Result<Self::Outcome, DispatchError>;
}

pub trait UpdatePetition<VoteId, AccountId, Hash, SignerView>:
    SignPetition<VoteId, AccountId, Hash, SignerView>
{
    fn update_petition(
        petition_id: VoteId,
        new_topic: Hash,
        clear_previous_vote_state: bool,
    ) -> DispatchResult;
}

// ====== Vote Logic ======

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
pub trait OpenVote<OrgId, Threshold, BlockNumber, VoteId>: GetVoteOutcome<VoteId> {
    fn open_vote(
        organization: OrgId,
        threshold: Threshold,
        duration: Option<BlockNumber>,
    ) -> Result<VoteId, DispatchError>;
}

pub trait Approved {
    fn approved(&self) -> bool;
}
pub trait Rejected {
    fn rejected(&self) -> bool;
}
pub trait Apply<Vote>: Sized {
    fn apply(&self, vote: Vote) -> Self;
}
pub trait Revert<Vote>: Sized {
    fn revert(&self, vote: Vote) -> Self;
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
    type State: Approved + Apply<Self::Vote> + Revert<Self::Vote>;
    fn apply_vote(
        state: Self::State,
        vote_magnitude: Self::Signal,
        new_vote_view: Self::Direction,
        old_vote_view: Self::Direction,
    ) -> Self::State;
}

pub trait CheckVoteStatus<Hash, VoteId>: ApplyVote<Hash> + GetVoteOutcome<VoteId> {
    fn check_vote_outcome(state: Self::State) -> Option<Self::Outcome>;
    fn check_vote_expired(state: &Self::State) -> bool;
}

pub trait MintableSignal<AccountId, OrgId, Threshold, BlockNumber, VoteId, Hash>:
    OpenVote<OrgId, Threshold, BlockNumber, VoteId> + ApplyVote<Hash>
{
    fn mint_custom_signal_for_account(vote_id: VoteId, who: &AccountId, signal: Self::Signal);

    fn batch_mint_signal(
        vote_id: VoteId,
        organization: OrgId,
    ) -> Result<Self::Signal, DispatchError>;
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
    OpenVote<OrgId, Threshold, BlockNumber, VoteId> + CheckVoteStatus<Hash, VoteId>
{
    fn vote_on_proposal(
        vote_id: VoteId,
        voter: AccountId,
        direction: Self::Direction,
        justification: Option<Hash>,
    ) -> DispatchResult;
}

// ~~~~~~~~ Bank Module ~~~~~~~~
use crate::bank::OnChainTreasuryID;
pub trait OnChainBank {
    type TreasuryId: Clone + From<OnChainTreasuryID>;
    type AssociatedId;
}
pub trait RegisterAccount<OrgId, AccountId, GovernanceConfig, Currency>: OnChainBank {
    // requires a deposit of some size above the minimum and returns the OnChainTreasuryID
    fn register_account(
        owners: OrgId,
        from: AccountId,
        amount: Currency,
        owner_s: GovernanceConfig,
    ) -> Result<Self::TreasuryId, DispatchError>;
    fn verify_owner(bank_id: Self::TreasuryId, org: OrgId) -> bool;
} // people should be eventually able to solicit loans from others to SEED a bank account but they cede some or all of the control...

pub trait CalculateOwnership<OrgId, AccountId, GovernanceConfig, Currency, FineArithmetic>:
    RegisterAccount<OrgId, AccountId, GovernanceConfig, Currency>
{
    fn calculate_proportion_ownership_for_account(
        account: AccountId,
        group: GovernanceConfig,
    ) -> Result<FineArithmetic, DispatchError>;
    fn calculate_proportional_amount_for_account(
        amount: Currency,
        account: AccountId,
        group: GovernanceConfig,
    ) -> Result<Currency, DispatchError>;
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
pub trait DepositsAndSpends<Currency> {
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
pub trait CheckBankBalances<Currency>: OnChainBank + DepositsAndSpends<Currency> {
    // prefer this method in most cases because
    fn get_bank_store(bank_id: Self::TreasuryId) -> Option<Self::Bank>;
    // -> invariant for module is that this returns the same as if you calculate total balance from the above storage item
    fn calculate_total_bank_balance_from_balances(bank_id: Self::TreasuryId) -> Option<Currency>;
}

pub trait DepositIntoBank<OrgId, AccountId, GovernanceConfig, Currency, Hash>:
    RegisterAccount<OrgId, AccountId, GovernanceConfig, Currency> + DepositsAndSpends<Currency>
{
    // get the bank corresponding to bank_id call infallible deposit
    // - only fails if `from` doesn't have enough Currency
    fn deposit_into_bank(
        from: AccountId,
        to_bank_id: Self::TreasuryId,
        amount: Currency,
        reason: Hash,
    ) -> Result<Self::AssociatedId, DispatchError>; // returns DepositId
}

pub trait DefaultBankPermissions<OrgId, AccountId, Currency, WithdrawalPermissions>:
    DepositsAndSpends<Currency> + OnChainBank
{
    fn can_register_account(account: AccountId, on_behalf_of: OrgId) -> bool;
    fn withdrawal_permissions_satisfy_org_standards(
        org: OrgId,
        withdrawal_permissions: WithdrawalPermissions,
    ) -> bool;
    fn can_reserve_for_spend(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
    fn can_commit_reserved_spend_for_transfer(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
    fn can_unreserve_uncommitted_to_make_free(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
    fn can_unreserve_committed_to_make_free(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
    fn can_transfer_spending_power(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
    fn can_commit_and_transfer_spending_power(
        account: AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError>;
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
pub trait ReservationMachine<OrgId, AccountId, GovernanceConfig, Currency, Hash>:
    RegisterAccount<OrgId, AccountId, GovernanceConfig, Currency>
{
    fn reserve_for_spend(
        bank_id: Self::TreasuryId,
        reason: Hash,
        amount: Currency,
        // acceptance committee for approving set aside spends below the amount
        controller: GovernanceConfig,
    ) -> Result<Self::AssociatedId, DispatchError>;
    fn commit_reserved_spend_for_transfer(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: Currency,
    ) -> DispatchResult;
    // bank controller can unreserve if not committed
    fn unreserve_uncommitted_to_make_free(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: Currency,
    ) -> DispatchResult;
    // reservation.controller() can unreserve committed funds
    fn unreserve_committed_to_make_free(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: Currency,
    ) -> DispatchResult;
    // reservation.controller() transfers control power to new_controller and enables liquidity by this controller
    fn transfer_spending_power(
        bank_id: Self::TreasuryId,
        reason: Hash,
        // reference to specific reservation
        reservation_id: Self::AssociatedId,
        amount: Currency,
        // move control of funds to new outer group which can reserve or withdraw directly
        new_controller: GovernanceConfig,
    ) -> Result<Self::AssociatedId, DispatchError>; // returns transfer_id
}

pub trait CommitAndTransfer<OrgId, AccountId, GovernanceConfig, Currency, Hash>:
    ReservationMachine<OrgId, AccountId, GovernanceConfig, Currency, Hash>
{
    // in one step
    fn commit_and_transfer_spending_power(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        reason: Hash,
        amount: Currency,
        new_controller: GovernanceConfig,
    ) -> Result<Self::AssociatedId, DispatchError>;
}

pub trait ExecuteSpends<OrgId, AccountId, GovernanceConfig, Currency, Hash>:
    OnChainBank + ReservationMachine<OrgId, AccountId, GovernanceConfig, Currency, Hash>
{
    fn spend_from_free(
        from_bank_id: Self::TreasuryId,
        to: AccountId,
        amount: Currency,
    ) -> DispatchResult;
    fn spend_from_transfers(
        from_bank_id: Self::TreasuryId,
        // transfer_id
        id: Self::AssociatedId,
        to: AccountId,
        amount: Currency,
    ) -> Result<Currency, DispatchError>;
}

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
    fn register_foundation_from_deposit(
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

// used for application and milestone review dispatch and diagnostics
pub trait StartReview<VoteID> {
    fn start_review(&self, vote_id: VoteID) -> Self;
    fn get_review_id(&self) -> Option<VoteID>;
}

pub trait ApproveWithoutTransfer {
    fn approve_without_transfer(&self) -> Self;
}

pub trait SetMakeTransfer<BankId, TransferId>: Sized {
    fn set_make_transfer(&self, bank_id: BankId, transfer_id: TransferId) -> Self;
    fn get_bank_id(&self) -> Option<BankId>;
    fn get_transfer_id(&self) -> Option<TransferId>;
}

pub trait StartTeamConsentPetition<Id, VoteID> {
    fn start_team_consent_petition(&self, org_id: Id, vote_id: VoteID) -> Self;
    fn get_team_consent_id(&self) -> Option<VoteID>;
    fn get_team_id(&self) -> Option<Id>;
}

// TODO: clean up the outer_flat_share_id dispatched for team consent if NOT formally approved
pub trait ApproveGrant<TeamID> {
    fn approve_grant(&self, team_id: TeamID) -> Self;
    fn get_full_team_id(&self) -> Option<TeamID>;
}
// TODO: RevokeApprovedGrant<VoteID> => vote to take away the team's grant and clean storage

pub trait SpendApprovedGrant<Currency>: Sized {
    fn spend_approved_grant(&self, amount: Currency) -> Option<Self>;
}

pub trait SubmitGrantApplication<Currency, AccountId, Hash>:
    CreateBounty<Currency, AccountId, Hash> + UseTermsOfAgreement<AccountId>
{
    type GrantApp: StartReview<Self::MultiVoteId>
        + StartTeamConsentPetition<Self::MultiShareId, Self::MultiVoteId>
        + ApproveGrant<Self::TeamId>;
    fn form_grant_application(
        caller: AccountId,
        bounty_id: u32,
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: Self::TermsOfAgreement,
    ) -> Result<Self::GrantApp, DispatchError>;
    fn submit_grant_application(
        caller: AccountId,
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
    type Milestone: StartReview<Self::MultiVoteId>
        + ApproveWithoutTransfer
        + SetMakeTransfer<Self::BankId, u32>;
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
        caller: AccountId,
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
        caller: AccountId,
        bounty_id: u32,
        milestone_id: u32,
    ) -> Result<Self::MilestoneState, DispatchError>;
}
