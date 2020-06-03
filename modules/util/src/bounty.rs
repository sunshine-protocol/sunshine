use crate::{
    bank::OnChainTreasuryID,
    organization::TermsOfAgreement, //ShareID
    traits::{
        ApproveGrant, ApproveWithoutTransfer, SetMakeTransfer, SpendApprovedGrant, StartReview,
        StartTeamConsentPetition,
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
    spend_reservation_id: u32,
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
        Hash: Parameter,
        Currency: Parameter,
        ReviewBoard: Clone,
    > BountyInformation<OrgId, Hash, Currency, ReviewBoard>
{
    // get OrgId for sponsor org basically
    pub fn foundation(&self) -> OrgId {
        self.foundation_id
    }
    pub fn bank_account(&self) -> OnChainTreasuryID {
        self.bank_account
    }
    pub fn spend_reservation(&self) -> u32 {
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
/// -> RULE: same org as bounty_info.foundation()
pub struct TeamID<
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    AccountId,
> {
    // organization identifier
    org: OrgId,
    // optional sudo (direction => revocable representative democracy)
    sudo: Option<AccountId>,
    // Vec<AccountId>, necessary for getting flat membership consent
    flat_membership: ShareId,
    // equity, necessary for constraining structured grant payouts
    weighted_membership: ShareId,
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone + PartialEq,
    > TeamID<OrgId, ShareId, AccountId>
{
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn flat_share_id(&self) -> ShareId {
        self.flat_membership
    }
    pub fn weighted_share_id(&self) -> ShareId {
        self.weighted_membership
    }
    pub fn is_sudo(&self, who: &AccountId) -> bool {
        if let Some(the_sudo) = &self.sudo {
            the_sudo == who
        } else {
            false
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Metadata that represents pre-dispatch, grant milestone reviews
pub enum ReviewBoard<
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    AccountId,
    Hash,
    WeightedThreshold,
> {
    /// Petition pre-call-metadata
    /// optional sudo, org_id, flat_share_id, signature_approval_threshold, signature_rejection_threshold, topic
    FlatPetitionReview(
        Option<AccountId>,
        OrgId,
        ShareId, // flat share id
        u32,
        Option<u32>,
        Option<Hash>,
    ),
    /// Vote-YesNo pre-call-metadata
    /// optional sudo, org_id, weighted_share_id, threshold expressed generically
    WeightedThresholdReview(
        Option<AccountId>,
        OrgId,
        ShareId,
        crate::voteyesno::SupportedVoteTypes,
        WeightedThreshold,
    ),
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: PartialEq,
        Hash,
        WeightedThreshold,
    > ReviewBoard<OrgId, ShareId, AccountId, Hash, WeightedThreshold>
{
    pub fn new_flat_petition_review(
        sudo: Option<AccountId>,
        org_id: OrgId,
        flat_share_id: ShareId,
        approval_threshold: u32,
        reject_threshold: Option<u32>,
        topic: Option<Hash>,
    ) -> ReviewBoard<OrgId, ShareId, AccountId, Hash, WeightedThreshold> {
        ReviewBoard::FlatPetitionReview(
            sudo,
            org_id,
            flat_share_id,
            approval_threshold,
            reject_threshold,
            topic,
        )
    }
    pub fn new_weighted_threshold_review(
        sudo: Option<AccountId>,
        org_id: OrgId,
        weighted_share_id: ShareId,
        vote_type: crate::voteyesno::SupportedVoteTypes,
        threshold: WeightedThreshold,
    ) -> ReviewBoard<OrgId, ShareId, AccountId, Hash, WeightedThreshold> {
        ReviewBoard::WeightedThresholdReview(sudo, org_id, weighted_share_id, vote_type, threshold)
    }
    pub fn is_sudo(&self, acc: &AccountId) -> bool {
        match self {
            ReviewBoard::FlatPetitionReview(Some(the_sudo), _, _, _, _, _) => the_sudo == acc,
            ReviewBoard::WeightedThresholdReview(Some(the_sudo), _, _, _, _) => the_sudo == acc,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Strongly typed vote identifier
pub enum VoteID {
    Petition(u32),
    Threshold(u32),
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum MilestoneStatus {
    SubmittedAwaitingResponse,
    SubmittedReviewStarted(VoteID),
    // if the milestone is approved but the approved application does not
    // have enough funds to satisfy milestone requirement, then this is set and we try again later...
    ApprovedButNotTransferred,
    // wraps Some(transfer_id) (bank_id is provided for convenient lookup, must equal bounty.bank)
    // None if the transfer wasn't able to be afforded at the time so it hasn't happened yet
    ApprovedAndTransferEnabled(OnChainTreasuryID, u32),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct MilestoneSubmission<Hash, Currency, AccountId> {
    submitter: AccountId,
    // the approved application from which the milestone derives its legitimacy
    referenced_application: u32,
    team_id: u32, // TODO: map from team_id => TeamID<OrgId, ShareId, AccountId>, adds a lookup but worth it long term IMO
    submission: Hash,
    amount: Currency,
    // the review status, none upon immediate submission
    state: MilestoneStatus,
}

impl<Hash: Clone, Currency: Clone, AccountId: Clone>
    MilestoneSubmission<Hash, Currency, AccountId>
{
    pub fn new(
        submitter: AccountId,
        referenced_application: u32,
        team_id: u32,
        submission: Hash,
        amount: Currency,
    ) -> MilestoneSubmission<Hash, Currency, AccountId> {
        MilestoneSubmission {
            submitter,
            referenced_application,
            team_id,
            submission,
            amount,
            state: MilestoneStatus::SubmittedAwaitingResponse,
        }
    }
    pub fn application_id(&self) -> u32 {
        self.referenced_application
    }
    pub fn team_id(&self) -> u32 {
        self.team_id
    }
    pub fn submission(&self) -> Hash {
        self.submission.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount.clone()
    }
    pub fn state(&self) -> MilestoneStatus {
        self.state
    }
    pub fn ready_for_review(&self) -> bool {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse => true,
            _ => false,
        }
    }
}

impl<Hash: Clone, Currency: Clone, AccountId: Clone> StartReview<VoteID>
    for MilestoneSubmission<Hash, Currency, AccountId>
{
    fn start_review(&self, vote_id: VoteID) -> Self {
        MilestoneSubmission {
            submitter: self.submitter.clone(),
            referenced_application: self.referenced_application,
            team_id: self.team_id,
            submission: self.submission.clone(),
            amount: self.amount.clone(),
            state: MilestoneStatus::SubmittedReviewStarted(vote_id),
        }
    }
    fn get_review_id(&self) -> Option<VoteID> {
        match self.state {
            MilestoneStatus::SubmittedReviewStarted(vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<Hash: Clone, Currency: Clone, AccountId: Clone> ApproveWithoutTransfer
    for MilestoneSubmission<Hash, Currency, AccountId>
{
    fn approve_without_transfer(&self) -> Self {
        MilestoneSubmission {
            submitter: self.submitter.clone(),
            referenced_application: self.referenced_application,
            team_id: self.team_id,
            submission: self.submission.clone(),
            amount: self.amount.clone(),
            state: MilestoneStatus::ApprovedButNotTransferred,
        }
    }
}

impl<Hash: Clone, Currency: Clone, AccountId: Clone> SetMakeTransfer<OnChainTreasuryID, u32>
    for MilestoneSubmission<Hash, Currency, AccountId>
{
    fn set_make_transfer(&self, bank_id: OnChainTreasuryID, transfer_id: u32) -> Self {
        MilestoneSubmission {
            submitter: self.submitter.clone(),
            referenced_application: self.referenced_application,
            team_id: self.team_id,
            submission: self.submission.clone(),
            amount: self.amount.clone(),
            state: MilestoneStatus::ApprovedAndTransferEnabled(bank_id, transfer_id),
        }
    }
    fn get_bank_id(&self) -> Option<OnChainTreasuryID> {
        match self.state {
            MilestoneStatus::ApprovedAndTransferEnabled(bank_id, _) => Some(bank_id),
            _ => None,
        }
    }
    fn get_transfer_id(&self) -> Option<u32> {
        match self.state {
            MilestoneStatus::ApprovedAndTransferEnabled(_, transfer_id) => Some(transfer_id),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum ApplicationState<ShareId: Codec + PartialEq + Zero + From<u32> + Copy> {
    SubmittedAwaitingResponse,
    // wraps a VoteId for the acceptance committee
    UnderReviewByAcceptanceCommittee(VoteID),
    // includes the flat_share_id, and the
    ApprovedByFoundationAwaitingTeamConsent(ShareId, VoteID),
    // counter is associated with a map in storage -> Some(TeamID<_, _, _>)
    ApprovedAndLive(u32),
    // closed for some reason
    Closed,
}

impl<ShareId: Codec + PartialEq + Zero + From<u32> + Copy> ApplicationState<ShareId> {
    // basically, can be approved (notably not when already approved)
    pub fn live(&self) -> bool {
        match self {
            ApplicationState::SubmittedAwaitingResponse => true,
            ApplicationState::UnderReviewByAcceptanceCommittee(_) => true,
            _ => false,
        }
    }
    pub fn matches_registered_team(&self, team_id: u32) -> bool {
        match self {
            ApplicationState::ApprovedAndLive(tid) => tid == &team_id,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<
    ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    AccountId,
    Shares,
    Currency,
    Hash,
> {
    /// Useful and necessary metadata
    submitter: AccountId,
    /// The ipfs reference to the application information
    description: Hash,
    /// total amount
    total_amount: Currency,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
    /// state of the application
    state: ApplicationState<ShareId>,
}

impl<
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
    > GrantApplication<ShareId, AccountId, Shares, Currency, Hash>
{
    pub fn new(
        submitter: AccountId,
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
    ) -> GrantApplication<ShareId, AccountId, Shares, Currency, Hash> {
        GrantApplication {
            submitter,
            description,
            total_amount,
            terms_of_agreement,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn state(&self) -> ApplicationState<ShareId> {
        self.state.clone()
    }
    pub fn total_amount(&self) -> Currency {
        self.total_amount.clone()
    }
    pub fn terms_of_agreement(&self) -> TermsOfAgreement<AccountId, Shares> {
        self.terms_of_agreement.clone()
    }
}

impl<
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
    > StartReview<VoteID> for GrantApplication<ShareId, AccountId, Shares, Currency, Hash>
{
    fn start_review(&self, vote_id: VoteID) -> Self {
        GrantApplication {
            submitter: self.submitter.clone(),
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::UnderReviewByAcceptanceCommittee(vote_id),
        }
    }
    fn get_review_id(&self) -> Option<VoteID> {
        match self.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        FlatShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
    > StartTeamConsentPetition<FlatShareId, VoteID>
    for GrantApplication<FlatShareId, AccountId, Shares, Currency, Hash>
{
    fn start_team_consent_petition(&self, share_id: FlatShareId, vote_id: VoteID) -> Self {
        // could type check the flat_share_id and vote_petition_id
        GrantApplication {
            submitter: self.submitter.clone(),
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::ApprovedByFoundationAwaitingTeamConsent(share_id, vote_id),
        }
    }
    fn get_team_flat_id(&self) -> Option<FlatShareId> {
        match self.state() {
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(share_id, _) => {
                Some(share_id)
            }
            _ => None,
        }
    }
    fn get_team_consent_id(&self) -> Option<VoteID> {
        match self.state() {
            ApplicationState::ApprovedByFoundationAwaitingTeamConsent(_, vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone,
        Hash: Clone,
    > ApproveGrant<u32> for GrantApplication<ShareId, AccountId, Shares, Currency, Hash>
{
    fn approve_grant(&self, team_id: u32) -> Self {
        GrantApplication {
            submitter: self.submitter.clone(),
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::ApprovedAndLive(team_id),
        }
    }
    fn get_full_team_id(&self) -> Option<u32> {
        match self.state() {
            ApplicationState::ApprovedAndLive(team_id) => Some(team_id),
            _ => None,
        }
    }
}

impl<
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: Clone,
        Shares: Clone,
        Currency: Clone + sp_std::ops::Sub<Currency, Output = Currency> + PartialOrd,
        Hash: Clone,
    > SpendApprovedGrant<Currency>
    for GrantApplication<ShareId, AccountId, Shares, Currency, Hash>
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
