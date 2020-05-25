use crate::{bank::OnChainTreasuryID, organization::TermsOfAgreement};
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
    pub fn claimed_funding_available(&self) -> Currency {
        self.claimed_funding_available.clone()
    }
    pub fn acceptance_committee(&self) -> ReviewBoard {
        self.acceptance_committee.clone()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Metadata that represents pre-dispatch, grant milestone reviews
/// - TODO: build a variant in which one person is sudo but the role is revocable and can be reassigned
pub enum ReviewBoard {
    /// Uses petition but only requires a single approver from the group
    /// - anyone can veto as well
    SimpleFlatReview(u32, u32),
    /// Weighted threshold
    WeightedThresholdReview(u32, u32, u32),
} //PetitionReview(u32, u32, u32, u32, u32)

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
pub enum ApplicationState {
    SubmittedAwaitingResponse,
    // wraps a VoteId for the acceptance committee
    UnderReviewByAcceptanceCommittee(u32),
    // however many individuals are left that need to consent
    ApprovedByFoundationAwaitingTeamConsent,
    // current milestone identifier
    ApprovedAndLive(u32),
    // closed for some reason
    Closed,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<AccountId, Currency, Hash> {
    /// The ipfs reference to the application information
    description: Hash,
    /// total amount
    total_amount: Currency,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    terms_of_agreement: TermsOfAgreement<AccountId>,
    /// state of the application
    state: ApplicationState,
}

impl<AccountId: Clone, Currency: Clone, Hash: Clone> GrantApplication<AccountId, Currency, Hash> {
    pub fn new(
        description: Hash,
        total_amount: Currency,
        terms_of_agreement: TermsOfAgreement<AccountId>,
    ) -> GrantApplication<AccountId, Currency, Hash> {
        GrantApplication {
            description,
            total_amount,
            terms_of_agreement,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn state(&self) -> ApplicationState {
        self.state
    }
    pub fn total_amount(&self) -> Currency {
        self.total_amount.clone()
    }
    pub fn terms(&self) -> TermsOfAgreement<AccountId> {
        self.terms_of_agreement.clone()
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
