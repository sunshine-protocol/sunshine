use crate::{
    bank::OnChainTreasuryID,
    organization::TermsOfAgreement,
    traits::{
        ApproveGrant, ApproveWithoutTransfer, GetTeamOrg, SetMakeTransfer, SpendApprovedGrant,
        StartReview, StartTeamConsentPetition,
    },
};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum BountyMapID {
    ApplicationId,
    MilestoneId,
}

impl Default for BountyMapID {
    fn default() -> BountyMapID {
        BountyMapID::ApplicationId
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The information most often read after a specific bounty is GOT
pub struct BountyInformation<
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ReservationId,
    Hash,
    Currency,
    ReviewBoard,
> {
    // Storage cid
    // - title, description, team requirements (all subjective metadata uses one reference)
    description: Hash,
    // registered organization associated with bounty
    foundation_id: OrgId,
    // On chain bank account associated with this bounty
    bank_account: OnChainTreasuryID,
    // Spend reservation identifier for funds set aside for bounty
    // TODO: update when funds are spent and a new spend reservation is required (gc)
    spend_reservation_id: ReservationId,
    // Collateral amount in the bank account (TODO: refresh method for syncing balance)
    funding_reserved: Currency,
    // Amount claimed to have on hand to fund projects related to the bounty
    // - used to derive the collateral ratio for this bounty, which must be above the module lower bound
    claimed_funding_available: Currency,
    // Committee metadata for approving an application
    acceptance_committee: ReviewBoard,
    // Committee metadata for approving milestones
    // -- if None, same as acceptance_committee by default
    supervision_committee: Option<ReviewBoard>,
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ReservationId: Codec + PartialEq + Zero + From<u32> + Copy,
        Hash: Parameter,
        Currency: Parameter,
        ReviewBoard: Clone,
    > BountyInformation<OrgId, ReservationId, Hash, Currency, ReviewBoard>
{
    // get OrgId for sponsor org basically
    pub fn foundation(&self) -> OrgId {
        self.foundation_id
    }
    pub fn bank_account(&self) -> OnChainTreasuryID {
        self.bank_account
    }
    pub fn spend_reservation(&self) -> ReservationId {
        self.spend_reservation_id
    }
    pub fn claimed_funding_available(&self) -> Currency {
        self.claimed_funding_available.clone()
    }
    pub fn acceptance_committee(&self) -> ReviewBoard {
        self.acceptance_committee.clone()
    }
    pub fn supervision_committee(&self) -> Option<ReviewBoard> {
        self.supervision_committee.clone()
    }
}

#[derive(new, PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Identifier for each registered team
pub struct TeamID<OrgId, AccountId> {
    /// Optional sudo (direction => revocable representative democracy)
    /// -> may not be same as self.org.sudo() but will be by default if not otherwise set
    sudo: Option<AccountId>,
    /// Organization identifier
    org: OrgId,
}

impl<OrgId: Codec + PartialEq + Zero + From<u32> + Copy, AccountId: Clone + PartialEq>
    TeamID<OrgId, AccountId>
{
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn is_sudo(&self, who: &AccountId) -> bool {
        if let Some(the_sudo) = &self.sudo {
            the_sudo == who
        } else {
            false
        }
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Vote call metadata, pre-call to dispatch votes
pub struct ReviewBoard<OrgId, AccountId, Hash, Threshold> {
    topic: Option<Hash>,
    sudo: Option<AccountId>,
    organization: OrgId,
    threshold: Threshold,
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: PartialEq,
        Hash: Clone,
        Threshold: Clone,
    > ReviewBoard<OrgId, AccountId, Hash, Threshold>
{
    pub fn topic(&self) -> Option<Hash> {
        self.topic.clone()
    }
    pub fn org(&self) -> OrgId {
        self.organization
    }
    pub fn threshold(&self) -> Threshold {
        self.threshold.clone()
    }
    pub fn is_sudo(&self, acc: &AccountId) -> bool {
        if let Some(the_sudo) = &self.sudo {
            the_sudo == acc
        } else {
            false
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum MilestoneStatus<OrgId, VoteId, TransferId> {
    SubmittedAwaitingResponse(OrgId),
    SubmittedReviewStarted(OrgId, VoteId),
    // if the milestone is approved but the approved application does not
    // have enough funds to satisfy milestone requirement, then this is set and we try again later...
    ApprovedButNotTransferred(OrgId),
    // wraps Some(transfer_id) (bank_id is proVoteIded for convenient lookup, must equal bounty.bank)
    // None if the transfer wasn't able to be afforded at the time so it hasn't happened yet
    ApprovedAndTransferEnabled(OnChainTreasuryID, TransferId),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct MilestoneSubmission<Hash, Currency, AccountId, BountyId, MilestoneStatus> {
    submitter: AccountId,
    // the approved application from which the milestone derives its legitimacy
    referenced_application: BountyId,
    submission: Hash,
    amount: Currency,
    // the review status, none upon immediate submission
    state: MilestoneStatus,
}

impl<
        Hash: Clone,
        Currency: Clone,
        AccountId: Clone,
        BountyId: Codec + Copy,
        TransferId: Codec + Copy,
        OrgId: Codec + Copy,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    >
    MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    >
{
    pub fn new(
        team_org: OrgId,
        submitter: AccountId,
        referenced_application: BountyId,
        submission: Hash,
        amount: Currency,
    ) -> MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    > {
        MilestoneSubmission {
            submitter,
            referenced_application,
            submission,
            amount,
            state: MilestoneStatus::SubmittedAwaitingResponse(team_org),
        }
    }
    pub fn application_id(&self) -> BountyId {
        self.referenced_application
    }
    pub fn submission(&self) -> Hash {
        self.submission.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn state(&self) -> MilestoneStatus<OrgId, VoteId, TransferId> {
        self.state
    }
    pub fn ready_for_review(&self) -> bool {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse(_) => true,
            _ => false,
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone,
        AccountId: Clone,
        BountyId: Codec + Copy,
        TransferId: Codec + Copy,
        OrgId: Codec + Copy,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > GetTeamOrg<OrgId>
    for MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    >
{
    fn get_team_org(&self) -> Option<OrgId> {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse(org_id) => Some(org_id),
            MilestoneStatus::SubmittedReviewStarted(org_id, _) => Some(org_id),
            MilestoneStatus::ApprovedButNotTransferred(org_id) => Some(org_id),
            _ => None,
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone,
        AccountId: Clone,
        BountyId: Codec + Copy,
        TransferId: Codec + Copy,
        OrgId: Codec + Copy,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > StartReview<VoteId>
    for MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    >
{
    fn start_review(&self, vote_id: VoteId) -> Option<Self> {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse(org_id) => Some(MilestoneSubmission {
                submitter: self.submitter.clone(),
                referenced_application: self.referenced_application,
                submission: self.submission.clone(),
                amount: self.amount.clone(),
                state: MilestoneStatus::SubmittedReviewStarted(org_id, vote_id),
            }),
            _ => None,
        }
    }
    fn get_review_id(&self) -> Option<VoteId> {
        match self.state {
            MilestoneStatus::SubmittedReviewStarted(_, vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone,
        AccountId: Clone,
        BountyId: Codec + Copy,
        TransferId: Codec + Copy,
        OrgId: Codec + Copy,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > ApproveWithoutTransfer
    for MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    >
{
    fn approve_without_transfer(&self) -> Option<Self> {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse(org_id) => Some(MilestoneSubmission {
                submitter: self.submitter.clone(),
                referenced_application: self.referenced_application,
                submission: self.submission.clone(),
                amount: self.amount.clone(),
                state: MilestoneStatus::ApprovedButNotTransferred(org_id),
            }),
            _ => None,
        }
    }
}

impl<
        Hash: Clone,
        Currency: Clone,
        AccountId: Clone,
        BountyId: Codec + Copy,
        TransferId: Codec + Copy,
        OrgId: Codec + Copy,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > SetMakeTransfer<OnChainTreasuryID, TransferId>
    for MilestoneSubmission<
        Hash,
        Currency,
        AccountId,
        BountyId,
        MilestoneStatus<OrgId, VoteId, TransferId>,
    >
{
    fn set_make_transfer(
        &self,
        bank_id: OnChainTreasuryID,
        transfer_id: TransferId,
    ) -> Option<Self> {
        match self.state {
            MilestoneStatus::SubmittedReviewStarted(_, _) => Some(MilestoneSubmission {
                submitter: self.submitter.clone(),
                referenced_application: self.referenced_application,
                submission: self.submission.clone(),
                amount: self.amount.clone(),
                state: MilestoneStatus::ApprovedAndTransferEnabled(bank_id, transfer_id),
            }),
            MilestoneStatus::ApprovedButNotTransferred(_) => Some(MilestoneSubmission {
                submitter: self.submitter.clone(),
                referenced_application: self.referenced_application,
                submission: self.submission.clone(),
                amount: self.amount.clone(),
                state: MilestoneStatus::ApprovedAndTransferEnabled(bank_id, transfer_id),
            }),
            _ => None,
        }
    }
    fn get_bank_id(&self) -> Option<OnChainTreasuryID> {
        match self.state {
            MilestoneStatus::ApprovedAndTransferEnabled(bank_id, _) => Some(bank_id),
            _ => None,
        }
    }
    fn get_transfer_id(&self) -> Option<TransferId> {
        match self.state {
            MilestoneStatus::ApprovedAndTransferEnabled(_, transfer_id) => Some(transfer_id),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum ApplicationState<TeamId, VoteId> {
    SubmittedAwaitingResponse,
    // wraps a vote_id for the acceptance committee
    UnderReviewByAcceptanceCommittee(VoteId),
    // wraps team_id, vote_id
    ApprovedByFoundationAwaitingTeamConsent(TeamId, VoteId),
    // team is working on this grant now under this organizational identifier
    ApprovedAndLive(TeamId),
    // closed for some reason
    Closed,
}

impl<TeamId: Clone, VoteId: Codec + PartialEq + Zero + From<u32> + Copy>
    ApplicationState<TeamId, VoteId>
{
    pub fn awaiting_review(&self) -> bool {
        match self {
            ApplicationState::SubmittedAwaitingResponse => true,
            _ => false,
        }
    }
    // basically, can be approved (notably not when already approved)
    pub fn under_review_by_acceptance_committee(self) -> Option<VoteId> {
        match self {
            ApplicationState::SubmittedAwaitingResponse => None,
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => Some(vote_id),
            _ => None,
        }
    }
    pub fn awaiting_team_consent(self) -> Option<(TeamId, VoteId)> {
        match self {
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(team_id, vote_id) => {
                Some((team_id, vote_id))
            }
            _ => None,
        }
    }
    pub fn approved_and_live(self) -> Option<TeamId> {
        match self {
            ApplicationState::ApprovedAndLive(tid) => Some(tid),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<
    AccountId,
    Shares,
    Currency,
    Hash,
    State, // generic to not add 2 more type params
> {
    /// The submitter is logged with submission
    submitter: AccountId,
    /// The ipfs reference to the application information
    description: Hash,
    /// total amount
    total_amount: Currency,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    terms_of_agreement: TermsOfAgreement<AccountId, Shares, Hash>,
    /// state of the application
    state: State,
}

impl<
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
        TeamId: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>>
{
    pub fn new(
        submitter: AccountId,
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: TermsOfAgreement<AccountId, Shares, Hash>,
    ) -> GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>> {
        GrantApplication {
            submitter,
            description,
            total_amount,
            terms_of_agreement,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn state(&self) -> ApplicationState<TeamId, VoteId> {
        self.state.clone()
    }
    pub fn total_amount(&self) -> Currency {
        self.total_amount.clone()
    }
    pub fn terms_of_agreement(&self) -> TermsOfAgreement<AccountId, Shares, Hash> {
        self.terms_of_agreement.clone()
    }
}

impl<
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
        TeamId: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > StartReview<VoteId>
    for GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>>
{
    fn start_review(&self, vote_id: VoteId) -> Option<Self> {
        match self.state {
            ApplicationState::SubmittedAwaitingResponse => Some(GrantApplication {
                submitter: self.submitter.clone(),
                description: self.description.clone(),
                total_amount: self.total_amount.clone(),
                terms_of_agreement: self.terms_of_agreement.clone(),
                state: ApplicationState::UnderReviewByAcceptanceCommittee(vote_id),
            }),
            _ => None,
        }
    }
    fn get_review_id(&self) -> Option<VoteId> {
        match self.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
        TeamId: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > StartTeamConsentPetition<TeamId, VoteId>
    for GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>>
{
    fn start_team_consent_petition(&self, team_id: TeamId, vote_id: VoteId) -> Option<Self> {
        match self.state {
            ApplicationState::UnderReviewByAcceptanceCommittee(_) => Some(GrantApplication {
                submitter: self.submitter.clone(),
                description: self.description.clone(),
                total_amount: self.total_amount.clone(),
                terms_of_agreement: self.terms_of_agreement.clone(),
                state: ApplicationState::ApprovedByFoundationAwaitingTeamConsent(team_id, vote_id),
            }),
            _ => None,
        }
    }
    fn get_team_id(&self) -> Option<TeamId> {
        match self.state() {
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(team_id, _) => Some(team_id),
            _ => None,
        }
    }
    fn get_team_consent_id(&self) -> Option<VoteId> {
        match self.state() {
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(_, vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
        TeamId: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > ApproveGrant<TeamId>
    for GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>>
{
    fn approve_grant(&self, team_id: TeamId) -> Self {
        GrantApplication {
            submitter: self.submitter.clone(),
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::ApprovedAndLive(team_id),
        }
    }
    fn get_full_team_id(&self) -> Option<TeamId> {
        match self.state() {
            ApplicationState::ApprovedAndLive(team_id) => Some(team_id),
            _ => None,
        }
    }
}

impl<
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone + sp_std::ops::Sub<Currency, Output = Currency> + PartialOrd,
        Hash: Clone,
        TeamId: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > SpendApprovedGrant<Currency>
    for GrantApplication<AccountId, Shares, Currency, Hash, ApplicationState<TeamId, VoteId>>
{
    fn spend_approved_grant(&self, amount: Currency) -> Option<Self> {
        match self.state {
            // grant must be in an approved state
            ApplicationState::ApprovedAndLive(_) => {
                // && amount must be below the grant application's amount
                if self.total_amount() >= amount {
                    let new_amount = self.total_amount() - amount;
                    Some(GrantApplication {
                        submitter: self.submitter.clone(),
                        description: self.description.clone(),
                        total_amount: new_amount,
                        terms_of_agreement: self.terms_of_agreement.clone(),
                        state: self.state.clone(),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// This struct is designed to track the payment for an ongoing bounty
pub struct BountyPaymentTracker<Currency> {
    received: Currency,
    due: Currency,
}

// upon posting a grant, the organization should assign reviewers for applications and state a formal review process for every bounty posted

// upon accepting a grant, the organization giving it should assign supervisors `=>` easy to make reviewers the supervisors
