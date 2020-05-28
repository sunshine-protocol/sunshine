use crate::{
    bank::OnChainTreasuryID,
    organization::{ShareID, TermsOfAgreement},
    traits::{ApproveGrant, StartApplicationReviewPetition, StartTeamConsentPetition},
};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
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

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The information most often read after a specific bounty is GOT
pub struct BountyInformation<Hash, Currency> {
    // Storage cid
    // - title, description, team requirements (all subjective metadata uses one reference)
    description: Hash,
    // registered organization associated with bounty
    foundation_id: u32,
    // On chain bank account associated with this bounty
    bank_account: OnChainTreasuryID,
    // Spend reservation identifier
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

impl<Hash: Parameter, Currency: Parameter> BountyInformation<Hash, Currency> {
    pub fn new(
        description: Hash,
        foundation_id: u32,
        bank_account: OnChainTreasuryID,
        spend_reservation_id: u32,
        funding_reserved: Currency,
        claimed_funding_available: Currency,
        acceptance_committee: ReviewBoard,
        supervision_committee: Option<ReviewBoard>,
    ) -> BountyInformation<Hash, Currency> {
        BountyInformation {
            description,
            foundation_id,
            bank_account,
            spend_reservation_id,
            funding_reserved,
            claimed_funding_available,
            acceptance_committee,
            supervision_committee,
        }
    }
    // get OrgId for sponsor org basically
    pub fn foundation(&self) -> u32 {
        self.foundation_id
    }
    pub fn claimed_funding_available(&self) -> Currency {
        self.claimed_funding_available.clone()
    }
    pub fn acceptance_committee(&self) -> ReviewBoard {
        self.acceptance_committee
    }
}

// - TODO: build a variant in which one person is sudo but the role is revocable and can be reassigned
// -> REUSE THIS LOGIC BETWEEN TEAMID and REVIEWBOARD
#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Metadata that represents pre-dispatch, grant milestone reviews
pub enum ReviewBoard {
    /// Uses petition but only requires a single approver from the group
    /// - anyone can veto as well
    SimpleFlatReview(u32, u32),
    /// Weighted threshold, Count(u32)
    WeightedThresholdReview(u32, u32),
} //PetitionReview(u32, u32, u32, u32, u32)

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Strongly typed vote identifier
pub enum VoteID {
    Petition(u32),
    Threshold(u32),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct MilestoneSubmission<Hash, Currency, Status> {
    // the team application for which this submission pertains
    application_id: u32,
    submission: Hash,
    amount: Currency,
    // the review status, none upon immediate submission
    review: Option<Status>,
}

impl<Hash, Currency, Status> MilestoneSubmission<Hash, Currency, Status> {
    pub fn new(
        application_id: u32,
        submission: Hash,
        amount: Currency,
    ) -> MilestoneSubmission<Hash, Currency, Status> {
        MilestoneSubmission {
            application_id,
            submission,
            amount,
            review: None,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Identifier for each registered team
/// -> RULE: same org as bounty_info.foundation()
pub struct TeamID<AccountId> {
    org: u32,
    // this should be optional and in the future, I want to orient it towards revocable representative democracy
    team_sudo: Option<AccountId>,
    flat_share_id: u32,
    weighted_share_id: u32,
}

impl<AccountId: Clone> TeamID<AccountId> {
    pub fn new(
        org: u32,
        team_sudo: Option<AccountId>,
        flat_share_id: u32,
        weighted_share_id: u32,
    ) -> TeamID<AccountId> {
        TeamID {
            org,
            team_sudo,
            flat_share_id,
            weighted_share_id,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ApplicationState<AccountId> {
    SubmittedAwaitingResponse,
    // wraps a VoteId for the acceptance committee
    UnderReviewByAcceptanceCommittee(VoteID),
    // includes the flat_share_id, and the
    ApprovedByFoundationAwaitingTeamConsent(ShareID, VoteID),
    // wraps outer_weighted_share_id associated with the team
    ApprovedAndLive(TeamID<AccountId>),
    // closed for some reason
    Closed,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<AccountId, Shares, Currency, Hash> {
    /// The ipfs reference to the application information
    description: Hash,
    /// total amount
    total_amount: Currency,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
    /// state of the application
    state: ApplicationState<AccountId>,
}

impl<AccountId: Clone, Shares: Clone, Currency: Clone, Hash: Clone>
    GrantApplication<AccountId, Shares, Currency, Hash>
{
    pub fn new(
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
    ) -> GrantApplication<AccountId, Shares, Currency, Hash> {
        GrantApplication {
            description,
            total_amount,
            terms_of_agreement,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn state(&self) -> ApplicationState<AccountId> {
        self.state.clone()
    }
    pub fn total_amount(&self) -> Currency {
        self.total_amount.clone()
    }
    pub fn terms_of_agreement(&self) -> TermsOfAgreement<AccountId, Shares> {
        self.terms_of_agreement.clone()
    }
}

impl<AccountId: Clone, Shares: Clone, Currency: Clone, Hash: Clone>
    StartApplicationReviewPetition<VoteID> for GrantApplication<AccountId, Shares, Currency, Hash>
{
    fn start_application_review_petition(&self, vote_id: VoteID) -> Self {
        GrantApplication {
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::UnderReviewByAcceptanceCommittee(vote_id),
        }
    }
    fn get_application_review_id(&self) -> Option<VoteID> {
        match self.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<AccountId: Clone, Shares: Clone, Currency: Clone, Hash: Clone>
    StartTeamConsentPetition<ShareID, VoteID>
    for GrantApplication<AccountId, Shares, Currency, Hash>
{
    fn start_team_consent_petition(&self, share_id: ShareID, vote_id: VoteID) -> Self {
        // could type check the flat_share_id and vote_petition_id
        GrantApplication {
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::ApprovedByFoundationAwaitingTeamConsent(share_id, vote_id),
        }
    }
    fn get_team_flat_id(&self) -> Option<ShareID> {
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

impl<AccountId: Clone, Shares: Clone, Currency: Clone, Hash: Clone> ApproveGrant<TeamID<AccountId>>
    for GrantApplication<AccountId, Shares, Currency, Hash>
{
    fn approve_grant(&self, team_id: TeamID<AccountId>) -> Self {
        GrantApplication {
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            terms_of_agreement: self.terms_of_agreement.clone(),
            state: ApplicationState::ApprovedAndLive(team_id),
        }
    }
    fn get_full_team_id(&self) -> Option<TeamID<AccountId>> {
        match self.state() {
            ApplicationState::ApprovedAndLive(team_id) => Some(team_id),
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
