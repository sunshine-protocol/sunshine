use crate::{bank::OnChainTreasuryID, organization::TermsOfAgreement, uuid::UUID2};
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::{PerThing, RuntimeDebug};
use sp_std::prelude::*;

pub type MilestoneId = u32;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct Requirements;
// impl some traits on this and use them to check the team's application

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The on-chain information for a bounty with keys (OrgId, BountyId)
pub struct BountyInformation<Hash, Currency, FineArithmetic> {
    // Storage cid
    description: Hash,
    // How the acceptance committee evaluates applications
    team_requirements: Option<Requirements>,
    // Committee metadata for approving an application
    acceptance_committee: VoteConfig<FineArithmetic>,
    // Committee metadata for approving milestones
    // -- if None, same as acceptance_committee by default
    supervision_committee: Option<VoteConfig<FineArithmetic>>,
    // On chain bank account associated with this bounty
    bank_account: OnChainTreasuryID,
    // Collateral amount in the bank account (TODO: refresh method for syncing balance)
    collateral: Currency,
    // Amount claimed to have on hand to fund projects related to the bounty
    // - used to derive the collateral ratio for this bounty, which must be above the module lower bound
    amount_promised: Currency,
}

impl<Hash: Parameter, Currency: Parameter, FineArithmetic: PerThing>
    BountyInformation<Hash, Currency, FineArithmetic>
{
    pub fn new(
        description: Hash,
        team_requirements: Option<Requirements>,
        acceptance_committee: VoteConfig<FineArithmetic>,
        supervision_committee: Option<VoteConfig<FineArithmetic>>,
        bank_account: OnChainTreasuryID,
        collateral: Currency,
        amount_promised: Currency,
    ) -> BountyInformation<Hash, Currency, FineArithmetic> {
        BountyInformation {
            description,
            team_requirements,
            acceptance_committee,
            supervision_committee,
            bank_account,
            collateral,
            amount_promised,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Metadata to be used to dispatch votes among share groups and/or request specific approval
pub enum VoteConfig<FineArithmetic> {
    /// (OrgId, ShareId) for all reviewers, (OrgId, ShareId) for SuperVoters (w/ veto power)
    PetitionReview(UUID2, UUID2),
    /// (OrgId, ShareId, Support Percentage Threshold, Turnout Percentage Threshold)
    YesNoPercentageThresholdVote(UUID2, FineArithmetic, FineArithmetic),
    /// (OrgId, ShareId, Support Count Threshold, Turnout Percentage Threshold)
    YesNoCountThresholdVote(UUID2, u32, u32),
    /// (OrgId, ShareId, Support 1P1V Count Threshold, Turnout 1P1V Count Threshold)
    YesNo1P1VCountThresholdVote(UUID2, u32, u32),
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct Task<Hash> {
    salt: u32,
    description: Hash,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// (BountyId, MilestoneId), Milestone => bool
/// - consider Option<Vec<Task<Hash>>> as value
pub struct Milestone<Hash, Currency> {
    team_id: UUID2, // org_id, share_id
    requirements: Hash,
    submission: Option<Hash>,
    reward: Currency,
    // replace with some STATE but need to be able to filter approved, waiting and done milestones
    // and to track payout specifically
    approved: bool,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The schedule for grant milestones
/// (OrgId, BountyId) => MilestoneSchedule
/// TODO: should be easy to pop a milestone from this vec and pop it onto completed in `BountyPaymentTracker`
pub struct MilestoneSchedule<Currency> {
    /// The sum of the rewards for all milestones in the other field
    total_reward: Currency,
    /// All the milestone identifiers for this MilestoneSchedule
    /// - not instantiated and added until they are approved
    milestones: Option<Vec<u32>>,
}

impl<Currency: Clone> MilestoneSchedule<Currency> {
    pub fn reward(&self) -> Currency {
        self.total_reward.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum ApplicationState {
    SubmittedAwaitingResponse,
    // however many individuals are left that need to consent
    ApprovedByFoundationAwaitingTeamConsent,
    // current milestone
    ApprovedAndLive(u32),
    // closed for some reason
    Closed,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// BountyId, GrantApplication => Option<ApplicationState>
pub struct GrantApplication<AccountId, Currency, Hash> {
    /// The ipfs reference to the application information
    description: Hash,
    /// The milestone proposed by the applying team, hashes need to be authenticated with data off-chain
    milestone_schedule: MilestoneSchedule<Currency>,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    terms_of_agreement: TermsOfAgreement<AccountId>,
}

impl<AccountId: Clone, Currency: Clone, Hash> GrantApplication<AccountId, Currency, Hash> {
    pub fn new(
        description: Hash,
        milestone_schedule: MilestoneSchedule<Currency>,
        terms_of_agreement: TermsOfAgreement<AccountId>,
    ) -> GrantApplication<AccountId, Currency, Hash> {
        GrantApplication {
            description,
            milestone_schedule,
            terms_of_agreement,
        }
    }
    pub fn milestones(&self) -> MilestoneSchedule<Currency> {
        self.milestone_schedule.clone()
    }
    pub fn terms(&self) -> TermsOfAgreement<AccountId> {
        self.terms_of_agreement.clone()
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// This struct is designed to track the payment for an ongoing bounty
pub struct BountyPaymentTracker<Currency> {
    /// Added once milestone is completed and removed once the recipient indicates they've
    /// received the payment
    due: Currency,
    /// Completed milestones
    completed: Vec<u32>,
    /// Milestones left
    schedule: MilestoneSchedule<Currency>,
}

// upon posting a grant, the organization should assign reviewers for applications and state a formal review process for every bounty posted

// upon accepting a grant, the organization giving it should assign supervisors `=>` easy to make reviewers the supervisors

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// This vote metadata describes the review of the milestone
/// - first the shareholder acknowledge the submission with submission hash
/// - then a vote is dispatched as per the review process
pub struct MilestoneReview {
    organization: u32,
    share_id: u32,
    support_requirement: u32,
    veto_rights: bool,
}
