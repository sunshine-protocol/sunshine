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

#[derive(new, PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub struct PercentageThreshold<Permill> {
    pct_to_pass: Permill,
    pct_to_fail: Option<Permill>,
}

impl<Permill: Copy> PercentageThreshold<Permill> {
    pub fn pct_to_pass(&self) -> Permill {
        self.pct_to_pass
    }
    pub fn pct_to_fail(&self) -> Option<Permill> {
        self.pct_to_fail
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResolutionMetadata<AccountId, OrgId, Threshold> {
    sudo: Option<AccountId>,
    org: OrgId,
    threshold: Threshold,
}

impl<AccountId: Clone + PartialEq, OrgId: Copy, Threshold: Copy>
    ResolutionMetadata<AccountId, OrgId, Threshold>
{
    pub fn sudo(&self) -> Option<AccountId> {
        self.sudo.clone()
    }
    pub fn is_sudo(&self, who: &AccountId) -> bool {
        if let Some(s) = self.sudo() {
            &s == who
        } else {
            false
        }
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn threshold(&self) -> Threshold {
        self.threshold
    }
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountyInformation<IpfsReference, AccountId, Currency, ReviewBoard> {
    // Storage cid
    info: IpfsReference,
    // Whoever posts the bounty
    poster: AccountId,
    // Funding reserved for this bounty
    funding_reserved: Currency,
    // Vote metadata for application approval
    permissions: ReviewBoard,
}

impl<
        IpfsReference: Clone,
        AccountId: Clone,
        Currency: Copy
            + PartialOrd
            + sp_std::ops::Sub<Output = Currency>
            + sp_std::ops::Add<Output = Currency>,
        ReviewBoard: Clone,
    > BountyInformation<IpfsReference, AccountId, Currency, ReviewBoard>
{
    pub fn info(&self) -> IpfsReference {
        self.info.clone()
    }
    pub fn poster(&self) -> AccountId {
        self.poster.clone()
    }
    pub fn funding_reserved(&self) -> Currency {
        self.funding_reserved
    }
    pub fn add_funding(&self, c: Currency) -> Self {
        BountyInformation {
            funding_reserved: self.funding_reserved + c,
            ..self.clone()
        }
    }
    pub fn pay_out_funding(&self, c: Currency) -> Self {
        let new_funding_reserved = self.funding_reserved() - c;
        BountyInformation {
            funding_reserved: new_funding_reserved,
            ..self.clone()
        }
    }
    pub fn permissions(&self) -> ReviewBoard {
        self.permissions.clone()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// All variants hold identifiers which point to larger objects in runtime storage maps
pub enum SubmissionState<BlockNumber, VoteId> {
    SubmittedAwaitingResponse,
    // approved but not executed
    ApprovedAndScheduled(BlockNumber),
    // wraps a vote_id for the acceptance committee
    ChallengedAndUnderReview(VoteId),
    // challenged and accepted again, scheduled for execution
    ApprovedAfterChallenge(BlockNumber),
    // closed for some reason, either approved or rejected
    Closed,
}

impl<
        BlockNumber: Copy + From<u32>,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    > SubmissionState<BlockNumber, VoteId>
{
    pub fn awaiting_review(&self) -> bool {
        matches!(self, SubmissionState::SubmittedAwaitingResponse)
    }
    pub fn approved_and_scheduled(&self) -> Option<BlockNumber> {
        match self {
            SubmissionState::ApprovedAndScheduled(n) => Some(*n),
            _ => None,
        }
    }
    pub fn under_review(&self) -> Option<VoteId> {
        match self {
            SubmissionState::ChallengedAndUnderReview(n) => Some(*n),
            _ => None,
        }
    }
    pub fn approved_after_challenge(&self) -> Option<BlockNumber> {
        match self {
            SubmissionState::ApprovedAfterChallenge(n) => Some(*n),
            _ => None,
        }
    }
    pub fn closed(&self) -> bool {
        matches!(self, SubmissionState::Closed)
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct BountySubmission<
    BountyId,
    AccountId,
    Org,
    IpfsReference,
    Currency,
    State,
> {
    /// The bounty for which this submission pertains
    bounty: BountyId,
    /// The submitter is logged with submission
    submitter: AccountId,
    /// The bank identifier for this team to receive funds that enforce an ownership structure based on share distribution
    org: Option<Org>,
    /// The IPFS reference to the application information
    submission_ref: IpfsReference,
    /// Total amount
    amount: Currency,
    /// State of the application
    state: State,
}

impl<
        BountyId: Copy,
        AccountId: Clone + PartialEq,
        Org: Copy,
        IpfsReference: Clone,
        Currency: Copy + PartialOrd + sp_std::ops::Sub<Output = Currency>,
        BlockNumber: Copy + From<u32>,
        VoteId: Codec + PartialEq + Zero + From<u32> + Copy,
    >
    BountySubmission<
        BountyId,
        AccountId,
        Org,
        IpfsReference,
        Currency,
        SubmissionState<BlockNumber, VoteId>,
    >
{
    pub fn new(
        bounty: BountyId,
        submitter: AccountId,
        org: Option<Org>,
        submission_ref: IpfsReference,
        amount: Currency,
    ) -> BountySubmission<
        BountyId,
        AccountId,
        Org,
        IpfsReference,
        Currency,
        SubmissionState<BlockNumber, VoteId>,
    > {
        BountySubmission {
            bounty,
            submitter,
            org,
            submission_ref,
            amount,
            state: SubmissionState::SubmittedAwaitingResponse,
        }
    }
    pub fn bounty(&self) -> BountyId {
        self.bounty
    }
    pub fn submitter(&self) -> AccountId {
        self.submitter.clone()
    }
    pub fn is_submitter(&self, who: &AccountId) -> bool {
        &self.submitter == who
    }
    pub fn submission(&self) -> IpfsReference {
        self.submission_ref.clone()
    }
    pub fn org(&self) -> Option<Org> {
        self.org
    }
    pub fn amount(&self) -> Currency {
        self.amount
    }
    pub fn pay_out_amount(&self, c: Currency) -> Option<Self> {
        if c <= self.amount() {
            let new_amount = self.amount() - c;
            Some(BountySubmission {
                amount: new_amount,
                ..self.clone()
            })
        } else {
            None
        }
    }
    pub fn state(&self) -> SubmissionState<BlockNumber, VoteId> {
        self.state
    }
    pub fn set_state(
        &self,
        state: SubmissionState<BlockNumber, VoteId>,
    ) -> Self {
        BountySubmission {
            state,
            ..self.clone()
        }
    }
    pub fn awaiting_review(&self) -> bool {
        self.state.awaiting_review()
    }
    pub fn approved_and_scheduled(&self) -> Option<BlockNumber> {
        self.state.approved_and_scheduled()
    }
    pub fn under_review(&self) -> Option<VoteId> {
        self.state.under_review()
    }
    pub fn approved_after_challenge(&self) -> Option<BlockNumber> {
        self.state.approved_after_challenge()
    }
    pub fn closed(&self) -> bool {
        self.state.closed()
    }
}
