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
pub struct Foundation<IpfsReference, Currency, Governance> {
    // Storage cid
    info: IpfsReference,
    // Raised amount
    funds: Currency,
    // Vote metadata for application approval
    gov: Governance,
}

impl<
        IpfsReference: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
        Governance: Clone,
    > Foundation<IpfsReference, Currency, Governance>
{
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn funds(&self) -> Currency {
        self.funds
    }
    pub fn add_funds(&self, a: Currency) -> Self {
        Foundation {
            funds: self.funds + a,
            ..self.clone()
        }
    }
    pub fn subtract_funds(&self, a: Currency) -> Self {
        Foundation {
            funds: self.funds - a,
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

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Recipient<AccountId, OrgId> {
    account: AccountId,
    org: Option<OrgId>,
}

impl<AccountId: Clone, OrgId: Copy> Recipient<AccountId, OrgId> {
    pub fn account(&self) -> AccountId {
        self.account.clone()
    }
    pub fn org(&self) -> Option<OrgId> {
        self.org
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct GrantApplication<
    FoundationId,
    IpfsReference,
    Recipient,
    Payment,
    State,
> {
    foundation_id: FoundationId,
    /// The IPFS reference to the application information
    submission_ref: IpfsReference,
    /// Recipient for the grant
    recipient: Recipient,
    /// Total amount requested (and its form)
    payment: Payment,
    /// State of the application
    state: State,
}

impl<
        FoundationId: Copy,
        IpfsReference: Clone,
        Recipient: Clone + PartialEq,
        Payment: Clone,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    >
    GrantApplication<
        FoundationId,
        IpfsReference,
        Recipient,
        Payment,
        ApplicationState<VoteId>,
    >
{
    pub fn new(
        foundation_id: FoundationId,
        submission_ref: IpfsReference,
        recipient: Recipient,
        payment: Payment,
    ) -> GrantApplication<
        FoundationId,
        IpfsReference,
        Recipient,
        Payment,
        ApplicationState<VoteId>,
    > {
        GrantApplication {
            foundation_id,
            submission_ref,
            recipient,
            payment,
            state: ApplicationState::SubmittedAwaitingResponse,
        }
    }
    pub fn foundation_id(&self) -> FoundationId {
        self.foundation_id
    }
    pub fn submission_ref(&self) -> IpfsReference {
        self.submission_ref.clone()
    }
    pub fn recipient(&self) -> Recipient {
        self.recipient.clone()
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
    Recipient,
    Payment,
    State,
> {
    base: (FoundationId, ApplicationId),
    submission: IpfsReference,
    recipient: Recipient,
    payment: Payment,
    // the review status, none upon immediate submission
    state: State,
}

impl<
        FoundationId: From<u32> + Copy,
        ApplicationId: Codec + Copy,
        IpfsReference: Clone,
        Recipient: Clone + PartialEq,
        Payment: Copy,
        VoteId: Codec + Copy,
    >
    MilestoneSubmission<
        FoundationId,
        ApplicationId,
        IpfsReference,
        Recipient,
        Payment,
        MilestoneStatus<VoteId>,
    >
{
    pub fn new(
        base: (FoundationId, ApplicationId),
        submission: IpfsReference,
        recipient: Recipient,
        payment: Payment,
    ) -> MilestoneSubmission<
        FoundationId,
        ApplicationId,
        IpfsReference,
        Recipient,
        Payment,
        MilestoneStatus<VoteId>,
    > {
        MilestoneSubmission {
            base,
            submission,
            recipient,
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
    pub fn recipient(&self) -> Recipient {
        self.recipient.clone()
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
