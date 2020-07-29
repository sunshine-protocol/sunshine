use codec::{
    Codec,
    Decode,
    Encode,
};
pub use sp_core::Hasher;
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Foundation<IpfsReference, AccountId, Currency, Governance> {
    // Storage cid
    info: IpfsReference,
    // Whoever deposits the bounty
    depositer: AccountId,
    // Deposit amount
    deposit: Currency,
    // Raised amount
    raised: Currency,
    // Vote metadata for application approval
    gov: Governance,
}

impl<
        IpfsReference: Clone,
        AccountId: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
        Governance: Clone,
    > Foundation<IpfsReference, AccountId, Currency, Governance>
{
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn depositer(&self) -> AccountId {
        self.depositer.clone()
    }
    pub fn deposit(&self) -> Currency {
        self.deposit
    }
    pub fn raised(&self) -> Currency {
        self.raised
    }
    pub fn add_raised(&self, a: Currency) -> Self {
        Foundation {
            raised: self.raised + a,
            ..self.clone()
        }
    }
    pub fn subtract_raised(&self, a: Currency) -> Self {
        Foundation {
            raised: self.raised - a,
            ..self.clone()
        }
    }
    pub fn gov(&self) -> Governance {
        self.gov.clone()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ApplicationState<VoteId> {
    SubmittedAwaitingResponse,
    // wraps a vote_id for the acceptance committee
    UnderReviewByAcceptanceCommittee(VoteId),
    ApprovedAndLive,
    Closed,
}

impl<VoteId: Codec + PartialEq + Zero + From<u32> + Copy>
    ApplicationState<VoteId>
{
    pub fn awaiting_review(&self) -> bool {
        matches!(self, ApplicationState::SubmittedAwaitingResponse)
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
        matches!(self, ApplicationState::ApprovedAndLive)
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<
    FoundationId,
    IpfsReference,
    AccountId,
    OrgId,
    Payment,
    State,
> {
    foundation: FoundationId,
    /// The IPFS reference to the application information
    submission: IpfsReference,
    /// The submitter is logged with submission
    submitter: AccountId,
    /// The org identifier for this team to receive funds that enforce an
    /// ownership structure based on share ownership
    team: Option<OrgId>,
    /// Total amount requested (and its form)
    payment: Payment,
    /// State of the application
    state: State,
}

impl<
        FoundationId: Copy,
        IpfsReference: Clone,
        AccountId: Clone + PartialEq,
        OrgId: Copy,
        Payment: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    >
    GrantApplication<
        FoundationId,
        IpfsReference,
        AccountId,
        OrgId,
        Payment,
        ApplicationState<VoteId>,
    >
{
    pub fn new(
        foundation: FoundationId,
        submission: IpfsReference,
        submitter: AccountId,
        team: Option<OrgId>,
        payment: Payment,
    ) -> GrantApplication<
        FoundationId,
        IpfsReference,
        AccountId,
        OrgId,
        Payment,
        ApplicationState<VoteId>,
    > {
        GrantApplication {
            foundation,
            submission,
            submitter,
            team,
            payment,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn foundation(&self) -> FoundationId {
        self.foundation
    }
    pub fn submission(&self) -> IpfsReference {
        self.submission.clone()
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn is_submitter(&self, who: &AccountId) -> bool {
        &self.submitter == who
    }
    pub fn team(&self) -> Option<OrgId> {
        self.team
    }
    pub fn payment(&self) -> Payment {
        self.payment.clone()
    }
    pub fn approved_and_live(&self) -> bool {
        self.state.approved_and_live()
    }
    pub fn under_review(&self) -> Option<VoteId> {
        self.state.under_review_by_acceptance_committee()
    }
    pub fn state(&self) -> ApplicationState<VoteId> {
        self.state
    }
    pub fn set_state(&self, s: ApplicationState<VoteId>) -> Self {
        GrantApplication {
            state: s,
            ..self.clone()
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
    FoundationId,
    ApplicationId,
    IpfsReference,
    AccountId,
    OrgId,
    Payment,
    State,
> {
    base: (FoundationId, ApplicationId),
    submission: IpfsReference,
    submitter: AccountId,
    team: Option<OrgId>,
    payment: Payment,
    // the review status, none upon immediate submission
    state: State,
}

impl<
        FoundationId: From<u32> + Copy,
        ApplicationId: Codec + Copy,
        IpfsReference: Clone,
        AccountId: Clone + PartialEq,
        OrgId: Copy,
        Payment: Copy,
        VoteId: Codec + Copy,
    >
    MilestoneSubmission<
        FoundationId,
        ApplicationId,
        IpfsReference,
        AccountId,
        OrgId,
        Payment,
        MilestoneStatus<VoteId>,
    >
{
    pub fn new(
        base: (FoundationId, ApplicationId),
        submission: IpfsReference,
        submitter: AccountId,
        team: Option<OrgId>,
        payment: Payment,
    ) -> MilestoneSubmission<
        FoundationId,
        ApplicationId,
        IpfsReference,
        AccountId,
        OrgId,
        Payment,
        MilestoneStatus<VoteId>,
    > {
        MilestoneSubmission {
            base,
            submission,
            submitter,
            team,
            payment,
            state: MilestoneStatus::SubmittedAwaitingResponse,
        }
    }
    pub fn base_foundation(&self) -> FoundationId {
        self.base.0
    }
    pub fn base_application(&self) -> ApplicationId {
        self.base.1
    }
    pub fn submission(&self) -> IpfsReference {
        self.submission.clone()
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn team(&self) -> Option<OrgId> {
        self.team
    }
    pub fn payment(&self) -> Payment {
        self.payment
    }
    pub fn state(&self) -> MilestoneStatus<VoteId> {
        self.state
    }
    pub fn ready_for_review(&self) -> bool {
        matches!(self.state, MilestoneStatus::SubmittedAwaitingResponse)
    }
    pub fn under_review(&self) -> Option<VoteId> {
        match self.state {
            MilestoneStatus::SubmittedReviewStarted(n) => Some(n),
            _ => None,
        }
    }
    pub fn approved_not_transferred(&self) -> bool {
        matches!(self.state, MilestoneStatus::ApprovedButNotTransferred)
    }
    pub fn approved_and_transferred(&self) -> bool {
        matches!(self.state, MilestoneStatus::ApprovedAndTransferExecuted)
    }
    pub fn set_state(&self, state: MilestoneStatus<VoteId>) -> Self {
        MilestoneSubmission {
            state,
            ..self.clone()
        }
    }
}
