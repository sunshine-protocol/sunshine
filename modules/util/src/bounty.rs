use crate::organization::TermsOfAgreement;
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

/*
An experiment to reduce the total number of generics by just type aliasing identifiers
instead of setting them as the module's associated type:
*/
pub type BountyId = u32;
pub type ApplicationId = u32;
pub type MilestoneId = u32;
pub type TaskId = u32;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct Requirements;
// impl some traits on this and use them to check the team's application

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// The on-chain information for a bounty with keys (OrgId, BountyId)
pub struct BountyInformation<Hash, Currency> {
    description: Hash,
    team_requirements: Option<Requirements>,
    pot: Currency,
}

impl<Hash: Parameter, Currency: Parameter> BountyInformation<Hash, Currency> {
    pub fn new(
        description: Hash,
        team_requirements: Option<Requirements>,
        pot: Currency,
    ) -> BountyInformation<Hash, Currency> {
        BountyInformation {
            description,
            team_requirements,
            pot,
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Each task
/// - TODO: add accountability such that there is some subset of the membership group
/// assigned to this task
pub struct Task<Hash> {
    id: TaskId,
    description: Hash,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// (OrgId, BountyId, MilestoneId) => Milestone
pub struct Milestone<Currency, Hash> {
    id: MilestoneId,
    description: Hash,
    reward: Currency,
    tasks: Vec<Task<Hash>>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The schedule for grant milestones
/// (OrgId, BountyId) => MilestoneSchedule
/// TODO: should be easy to pop a milestone from this vec and pop it onto completed in `BountyPaymentTracker`
pub struct MilestoneSchedule<Currency> {
    /// The sum of the rewards for all milestones in the other field
    total_reward: Currency,
    /// All the milestone identifiers for this MilestoneSchedule
    milestones: Vec<MilestoneId>,
}

impl<Currency: Copy> MilestoneSchedule<Currency> {
    pub fn reward(&self) -> Currency {
        self.total_reward
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// (OrgId, BountyId), ApplicationId => BountyApplication<AccountId, Shares, Currency, Hash>
pub struct BountyApplication<AccountId, Shares, Currency, Hash> {
    /// The description that goes
    description: Hash,
    /// The milestone proposed by the applying team, hashes need to be authenticated with data off-chain
    proposed_milestone_schedule: MilestoneSchedule<Currency>,
    /// The terms of agreement that must agreed to by all members before the bounty execution starts
    basic_terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
}

impl<AccountId, Shares, Currency, Hash> BountyApplication<AccountId, Shares, Currency, Hash> {
    pub fn new(
        description: Hash,
        proposed_milestone_schedule: MilestoneSchedule<Currency>,
        basic_terms_of_agreement: TermsOfAgreement<AccountId, Shares>,
    ) -> BountyApplication<AccountId, Shares, Currency, Hash> {
        BountyApplication {
            description,
            proposed_milestone_schedule,
            basic_terms_of_agreement,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// This struct is designed to track the payment for an ongoing bounty
pub struct BountyPaymentTracker<Currency> {
    /// Added once milestone is completed and removed once the recipient indicates they've
    /// received the payment
    due: Currency,
    /// Completed milestones
    completed: Vec<MilestoneId>,
    /// Milestones left
    schedule: MilestoneSchedule<Currency>,
}

// upon posting a grant, the organization should assign reviewers for applications and state a formal review process for every bounty posted

// upon accepting a grant, the organization giving it should assign supervisors `=>` easy to make reviewers the supervisors

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// This vote metadata describes the review of the milestone
/// - first the shareholder acknowledge the submission with submission hash
/// - then a vote is dispatched as per the review process
pub struct MilestoneReview<OrgId, ShareId> {
    organization: OrgId,
    share_id: ShareId,
    support_requirement: u32,
    veto_rights: bool,
}
