use codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountyInformation<IpfsReference, AccountId, Currency> {
    // Storage cid
    info: IpfsReference,
    // Whoever posts the bounty
    depositer: AccountId,
    // Total amount
    total: Currency,
}

impl<
        IpfsReference: Clone,
        AccountId: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
    > BountyInformation<IpfsReference, AccountId, Currency>
{
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

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum SubmissionState {
    SubmittedAwaitingResponse,
    ApprovedAndExecuted,
}

impl SubmissionState {
    pub fn awaiting_review(&self) -> bool {
        matches!(self, SubmissionState::SubmittedAwaitingResponse)
    }
    pub fn approved(&self) -> bool {
        matches!(self, SubmissionState::ApprovedAndExecuted)
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountySubmission<BountyId, IpfsReference, AccountId, Currency, State>
{
    /// The bounty for which this submission pertains
    bounty: BountyId,
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
        IpfsReference: Clone,
        AccountId: Clone + PartialEq,
        Currency: Copy + PartialOrd + sp_std::ops::Sub<Output = Currency>,
    >
    BountySubmission<
        BountyId,
        IpfsReference,
        AccountId,
        Currency,
        SubmissionState,
    >
{
    pub fn new(
        bounty: BountyId,
        submission_ref: IpfsReference,
        submitter: AccountId,
        amount: Currency,
    ) -> BountySubmission<
        BountyId,
        IpfsReference,
        AccountId,
        Currency,
        SubmissionState,
    > {
        BountySubmission {
            bounty,
            submission_ref,
            submitter,
            amount,
            state: SubmissionState::SubmittedAwaitingResponse,
        }
    }
    pub fn bounty_id(&self) -> BountyId {
        self.bounty
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
    pub fn awaiting_review(&self) -> bool {
        self.state.awaiting_review()
    }
}
