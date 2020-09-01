use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum BountyState<VoteId> {
    NoPendingChallenges,
    ChallengedToClose(VoteId),
}

impl<VoteId> Default for BountyState<VoteId> {
    fn default() -> BountyState<VoteId> {
        BountyState::NoPendingChallenges
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountyInfo2<IpfsReference, Governance, Currency, State> {
    // Storage cid
    info: IpfsReference,
    // Whoever posts the bounty
    gov: Governance,
    // Total amount
    total: Currency,
    // State
    state: State,
}

impl<
        IpfsReference: Clone,
        Governance: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
        VoteId: Copy,
    > BountyInfo2<IpfsReference, Governance, Currency, BountyState<VoteId>>
{
    pub fn new(info: IpfsReference, gov: Governance, total: Currency) -> Self {
        Self {
            info,
            gov,
            total,
            state: BountyState::default(),
        }
    }
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn gov(&self) -> Governance {
        self.gov.clone()
    }
    pub fn total(&self) -> Currency {
        self.total
    }
    pub fn state(&self) -> BountyState<VoteId> {
        self.state
    }
    pub fn set_state(&self, b: BountyState<VoteId>) -> Self {
        Self {
            state: b,
            ..self.clone()
        }
    }
    pub fn add_funds(&self, c: Currency) -> Self {
        Self {
            total: self.total + c,
            ..self.clone()
        }
    }
    pub fn subtract_funds(&self, c: Currency) -> Self {
        Self {
            total: self.total - c,
            ..self.clone()
        }
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountyInformation<BountyId, IpfsReference, AccountId, Currency> {
    // Bounty identifier (pre-hash key for storage value)
    id: BountyId,
    // Storage cid
    info: IpfsReference,
    // Whoever posts the bounty
    depositer: AccountId,
    // Total amount
    total: Currency,
}

impl<
        BountyId: Copy,
        IpfsReference: Clone,
        AccountId: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
    > BountyInformation<BountyId, IpfsReference, AccountId, Currency>
{
    pub fn id(&self) -> BountyId {
        self.id
    }
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn depositer(&self) -> AccountId {
        self.depositer.clone()
    }
    pub fn total(&self) -> Currency {
        self.total
    }
    pub fn add_total(&self, c: Currency) -> Self {
        BountyInformation {
            total: self.total + c,
            ..self.clone()
        }
    }
    pub fn subtract_total(&self, c: Currency) -> Self {
        BountyInformation {
            total: self.total - c,
            ..self.clone()
        }
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Contribution<BountyId, AccountId, Currency> {
    id: BountyId,
    account: AccountId,
    total: Currency,
}

impl<
        BountyId: Copy,
        AccountId: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
    > Contribution<BountyId, AccountId, Currency>
{
    pub fn id(&self) -> BountyId {
        self.id
    }
    pub fn account(&self) -> AccountId {
        self.account.clone()
    }
    pub fn total(&self) -> Currency {
        self.total
    }
    pub fn add_total(&self, c: Currency) -> Self {
        Self {
            total: self.total + c,
            ..self.clone()
        }
    }
    pub fn subtract_total(&self, c: Currency) -> Self {
        Self {
            total: self.total - c,
            ..self.clone()
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum SubmissionState {
    SubmittedAwaitingResponse,
    ApprovedAndExecuted,
}

impl Default for SubmissionState {
    fn default() -> SubmissionState {
        SubmissionState::SubmittedAwaitingResponse
    }
}

impl SubmissionState {
    pub fn awaiting_review(&self) -> bool {
        matches!(self, SubmissionState::SubmittedAwaitingResponse)
    }
    pub fn approved(&self) -> bool {
        matches!(self, SubmissionState::ApprovedAndExecuted)
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Submission state for Bounty2
pub enum SubmissionState2<BlockNumber, VoteId> {
    SubmittedAwaitingResponse,
    ApprovedAndScheduled(BlockNumber),
    ChallengedAndUnderReview(VoteId),
}

impl<BlockNumber: Copy, VoteId: Copy> Default
    for SubmissionState2<BlockNumber, VoteId>
{
    fn default() -> SubmissionState2<BlockNumber, VoteId> {
        SubmissionState2::SubmittedAwaitingResponse
    }
}

impl<BlockNumber: Copy, VoteId: Copy> SubmissionState2<BlockNumber, VoteId> {
    pub fn awaiting_review(&self) -> bool {
        matches!(self, SubmissionState2::SubmittedAwaitingResponse)
    }
    pub fn approved_and_scheduled(&self) -> Option<BlockNumber> {
        match self {
            SubmissionState2::ApprovedAndScheduled(n) => Some(*n),
            _ => None,
        }
    }
    pub fn under_review(&self) -> Option<VoteId> {
        match self {
            SubmissionState2::ChallengedAndUnderReview(n) => Some(*n),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountySubmission<
    BountyId,
    SubmissionId,
    IpfsReference,
    AccountId,
    Currency,
    State,
> {
    /// The identifiers for the parent bounty and this storage item
    id: (BountyId, SubmissionId),
    /// The IPFS reference to the application information
    submission_ref: IpfsReference,
    /// The submitter is logged with submission
    submitter: AccountId,
    /// Total amount
    amount: Currency,
    /// State of the application
    state: State,
}

impl<
        BountyId: Copy,
        SubmissionId: Copy,
        IpfsReference: Clone,
        AccountId: Clone + PartialEq,
        Currency: Copy + PartialOrd + sp_std::ops::Sub<Output = Currency>,
        State: Copy + Default,
    >
    BountySubmission<
        BountyId,
        SubmissionId,
        IpfsReference,
        AccountId,
        Currency,
        State,
    >
{
    pub fn new(
        bounty: BountyId,
        submission: SubmissionId,
        submission_ref: IpfsReference,
        submitter: AccountId,
        amount: Currency,
    ) -> BountySubmission<
        BountyId,
        SubmissionId,
        IpfsReference,
        AccountId,
        Currency,
        State,
    > {
        BountySubmission {
            id: (bounty, submission),
            submission_ref,
            submitter,
            amount,
            state: State::default(),
        }
    }
    pub fn bounty_id(&self) -> BountyId {
        self.id.0
    }
    pub fn submission_id(&self) -> SubmissionId {
        self.id.1
    }
    pub fn submission(&self) -> IpfsReference {
        self.submission_ref.clone()
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn is_submitter(&self, who: &AccountId) -> bool {
        &self.submitter == who
    }
    pub fn amount(&self) -> Currency {
        self.amount
    }
    pub fn pay_out_amount(&self, c: Currency) -> Self {
        let new_amount = self.amount() - c;
        BountySubmission {
            amount: new_amount,
            ..self.clone()
        }
    }
    pub fn state(&self) -> State {
        self.state
    }
}
