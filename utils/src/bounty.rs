use crate::traits::{
    ApproveGrant,
    ApproveWithoutTransfer,
    SpendApprovedGrant,
    StartReview,
};
use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
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
pub struct BountyInformation<Poster, Hash, Currency, ReviewBoard> {
    // Whoever posts the bounty, must be Into<AccountId> \forall variants of enum
    poster: Poster,
    // Storage cid
    topic: Hash,
    // Funding reserved for this bounty
    funding_reserved: Currency,
    // Vote metadata for application approval
    acceptance_committee: ReviewBoard,
    // Vote metadata for milestone approval
    supervision_committee: Option<ReviewBoard>,
}

impl<Poster: Clone, Hash: Clone, Currency: Copy, ReviewBoard: Clone>
    BountyInformation<Poster, Hash, Currency, ReviewBoard>
{
    pub fn poster(&self) -> Poster {
        self.poster.clone()
    }
    pub fn topic(&self) -> Hash {
        self.topic.clone()
    }
    pub fn funding_reserved(&self) -> Currency {
        self.funding_reserved
    }
    pub fn acceptance_committee(&self) -> ReviewBoard {
        self.acceptance_committee.clone()
    }
    pub fn supervision_committee(&self) -> Option<ReviewBoard> {
        self.supervision_committee.clone()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum ApplicationState<VoteId> {
    SubmittedAwaitingResponse,
    // wraps a vote_id for the acceptance committee
    UnderReviewByAcceptanceCommittee(VoteId),
    // wraps vote_id for which the team is consenting on this
    // ApprovedByFoundationAwaitingTeamConsent(VoteId),
    // team is working on this grant now
    ApprovedAndLive,
    // closed for some reason
    Closed,
}

impl<VoteId: Codec + PartialEq + Zero + From<u32> + Copy>
    ApplicationState<VoteId>
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
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => {
                Some(vote_id)
            }
            _ => None,
        }
    }
    pub fn approved_and_live(self) -> bool {
        match self {
            ApplicationState::ApprovedAndLive => true,
            _ => false,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<AccountId, BankId, Currency, Hash, State> {
    /// The submitter is logged with submission
    submitter: AccountId,
    /// The bank identifier for this team to receive funds that enforce an ownership structure based on share distribution
    bank: Option<BankId>,
    /// The IPFS reference to the application information
    description: Hash,
    /// Total amount
    total_amount: Currency,
    /// State of the application
    state: State,
}

impl<
        AccountId: Clone + PartialEq,
        BankId: Copy,
        Currency: Clone,
        Hash: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    >
    GrantApplication<
        AccountId,
        BankId,
        Currency,
        Hash,
        ApplicationState<VoteId>,
    >
{
    pub fn new(
        submitter: AccountId,
        bank: Option<BankId>,
        description: Hash,
        total_amount: Currency,
    ) -> GrantApplication<
        AccountId,
        BankId,
        Currency,
        Hash,
        ApplicationState<VoteId>,
    > {
        GrantApplication {
            submitter,
            bank,
            description,
            total_amount,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn is_submitter(&self, who: &AccountId) -> bool {
        &self.submitter == who
    }
    pub fn submission(&self) -> Hash {
        self.description.clone()
    }
    pub fn bank(&self) -> Option<BankId> {
        self.bank
    }
    pub fn state(&self) -> ApplicationState<VoteId> {
        self.state
    }
    pub fn total_amount(&self) -> Currency {
        self.total_amount.clone()
    }
}

impl<
        AccountId: Clone + PartialEq,
        BankId: Copy,
        Currency: Clone,
        Hash: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > StartReview<VoteId>
    for GrantApplication<
        AccountId,
        BankId,
        Currency,
        Hash,
        ApplicationState<VoteId>,
    >
{
    fn start_review(&self, vote_id: VoteId) -> Option<Self> {
        match self.state {
            ApplicationState::SubmittedAwaitingResponse => {
                Some(GrantApplication {
                    submitter: self.submitter.clone(),
                    bank: self.bank,
                    description: self.description.clone(),
                    total_amount: self.total_amount.clone(),
                    state: ApplicationState::UnderReviewByAcceptanceCommittee(
                        vote_id,
                    ),
                })
            }
            _ => None,
        }
    }
    fn get_review_id(&self) -> Option<VoteId> {
        match self.state() {
            ApplicationState::UnderReviewByAcceptanceCommittee(vote_id) => {
                Some(vote_id)
            }
            _ => None,
        }
    }
}

impl<
        AccountId: Clone + PartialEq,
        BankId: Copy,
        Currency: Clone,
        Hash: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > ApproveGrant
    for GrantApplication<
        AccountId,
        BankId,
        Currency,
        Hash,
        ApplicationState<VoteId>,
    >
{
    fn approve_grant(&self) -> Self {
        GrantApplication {
            submitter: self.submitter.clone(),
            bank: self.bank,
            description: self.description.clone(),
            total_amount: self.total_amount.clone(),
            state: ApplicationState::ApprovedAndLive,
        }
    }
    fn grant_approved(&self) -> bool {
        match self.state {
            ApplicationState::ApprovedAndLive => true,
            _ => false,
        }
    }
}

impl<
        AccountId: Clone + PartialEq,
        BankId: Copy,
        Currency: Copy + sp_std::ops::Sub<Currency, Output = Currency> + PartialOrd,
        Hash: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > SpendApprovedGrant<Currency>
    for GrantApplication<
        AccountId,
        BankId,
        Currency,
        Hash,
        ApplicationState<VoteId>,
    >
{
    fn spend_approved_grant(&self, amount: Currency) -> Option<Self> {
        match self.state {
            // grant must be in an approved state
            ApplicationState::ApprovedAndLive => {
                // && amount must be below the grant application's amount
                if self.total_amount >= amount {
                    let new_amount = self.total_amount - amount;
                    Some(GrantApplication {
                        submitter: self.submitter.clone(),
                        bank: self.bank,
                        description: self.description.clone(),
                        total_amount: new_amount,
                        state: self.state,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum MilestoneStatus<VoteId> {
    SubmittedAwaitingResponse,
    SubmittedReviewStarted(VoteId),
    ApprovedButNotTransferred,
    ApprovedAndTransferExecuted,
}

impl<VoteId> Default for MilestoneStatus<VoteId> {
    fn default() -> MilestoneStatus<VoteId> {
        MilestoneStatus::SubmittedAwaitingResponse
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct MilestoneSubmission<
    AccountId,
    ApplicationId,
    Hash,
    Currency,
    MilestoneStatus,
> {
    submitter: AccountId,
    // the approved application from which the milestone derives its legitimacy
    referenced_application: ApplicationId,
    submission: Hash,
    amount: Currency,
    // the review status, none upon immediate submission
    state: MilestoneStatus,
}

impl<
        AccountId: Clone + PartialEq,
        ApplicationId: Codec + Copy,
        Hash: Clone,
        Currency: Copy,
        VoteId: Codec + Copy,
    >
    MilestoneSubmission<
        AccountId,
        ApplicationId,
        Hash,
        Currency,
        MilestoneStatus<VoteId>,
    >
{
    pub fn new(
        submitter: AccountId,
        referenced_application: ApplicationId,
        submission: Hash,
        amount: Currency,
    ) -> MilestoneSubmission<
        AccountId,
        ApplicationId,
        Hash,
        Currency,
        MilestoneStatus<VoteId>,
    > {
        MilestoneSubmission {
            submitter,
            referenced_application,
            submission,
            amount,
            state: MilestoneStatus::SubmittedAwaitingResponse,
        }
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn referenced_application(&self) -> ApplicationId {
        self.referenced_application
    }
    pub fn submission(&self) -> Hash {
        self.submission.clone()
    }
    pub fn amount(&self) -> Currency {
        self.amount
    }
    pub fn state(&self) -> MilestoneStatus<VoteId> {
        self.state
    }
    pub fn set_state(&self, state: MilestoneStatus<VoteId>) -> Self {
        MilestoneSubmission {
            state,
            ..self.clone()
        }
    }
    pub fn ready_for_review(&self) -> bool {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse => true,
            _ => false,
        }
    }
}

impl<
        AccountId: Clone + PartialEq,
        ApplicationId: Codec + Copy,
        Hash: Clone,
        Currency: Copy,
        VoteId: Codec + Copy,
    > StartReview<VoteId>
    for MilestoneSubmission<
        AccountId,
        ApplicationId,
        Hash,
        Currency,
        MilestoneStatus<VoteId>,
    >
{
    fn start_review(&self, vote_id: VoteId) -> Option<Self> {
        match self.state {
            MilestoneStatus::SubmittedAwaitingResponse => {
                Some(MilestoneSubmission {
                    submitter: self.submitter.clone(),
                    referenced_application: self.referenced_application,
                    submission: self.submission.clone(),
                    amount: self.amount,
                    state: MilestoneStatus::SubmittedReviewStarted(vote_id),
                })
            }
            _ => None,
        }
    }
    fn get_review_id(&self) -> Option<VoteId> {
        match self.state {
            MilestoneStatus::SubmittedReviewStarted(vote_id) => Some(vote_id),
            _ => None,
        }
    }
}

impl<
        AccountId: Clone + PartialEq,
        ApplicationId: Codec + Copy,
        Hash: Clone,
        Currency: Copy,
        VoteId: Codec + Copy,
    > ApproveWithoutTransfer
    for MilestoneSubmission<
        AccountId,
        ApplicationId,
        Hash,
        Currency,
        MilestoneStatus<VoteId>,
    >
{
    fn approve_without_transfer(&self) -> Self {
        self.set_state(MilestoneStatus::ApprovedButNotTransferred)
    }
}
