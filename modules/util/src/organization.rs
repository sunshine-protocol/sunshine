use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Static terms of agreement, define how the enforced payout structure for grants
pub struct TermsOfAgreement<AccountId> {
    /// If Some(account), then account is the sudo for the duration of the grant
    supervisor: Option<AccountId>,
    /// The share allocation for metadata
    share_metadata: Vec<(AccountId, u32)>,
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Defined paths for how the terms of agreement can change
pub struct FullTermsOfAgreement<AccountId> {
    /// The starting state for the group
    basic_terms: TermsOfAgreement<AccountId>,
    /// This represents the metagovernance configuration, how the group can coordinate changes
    allowed_changes: Vec<(
        Catalyst<AccountId>,
        Option<RequiredVote>,
        Option<EnforcedOutcome<AccountId>>,
    )>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Authenticates that the given user can do the action in question to
/// trigger the `RequiredVote`
pub enum Catalyst<AccountId> {
    ReportBadBehavior(AccountId),
    SubmitMilestone(AccountId),
    RequestMilestoneAdjustment(AccountId),
    SwapRole(AccountId, AccountId),
} // TODO: upgrade path from suborganization to separate organization

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// TODO: Add VoteConfig = enum { 1p1v_count, 1p1v_percentage, share_weighted_count, share_weighted_percentage }
/// - each of which has two thresholds!
pub enum RequiredVote {
    /// Only one supervisor approval is required but everyone has veto rights
    OneSupervisorApprovalWithVetoRights(u32, u32),
    /// Two supervisor approvals is required but everyone has veto rights
    TwoSupervisorsApprovalWithVetoRights(u32, u32),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum EnforcedOutcome<AccountId> {
    /// Grant paid out as per bounty (hosting org, bounty recipient, milestone in question)
    /// (OrgId, ShareId, BountyId, MilestoneId)
    GrantPayoutBasedOnShareDistribution(u32, u32, u32, u32),
    /// Remove member for unacceptable behavior
    /// (OrgId, ShareId)
    RemoveMemberForBadBehavior(u32, u32, AccountId),
    /// Swap the first account for the second account in the same role for a grant team
    /// (OrgId, ShareId)
    SwapRoleOnGrantTeam(u32, u32, AccountId, AccountId),
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
// make it easy to verify existence in the context of Bank
pub enum ShareID {
    Flat(u32),
    WeightedAtomic(u32),
} // TODO: add `DivisibleShares` => Ranked Choice Voting

impl Default for ShareID {
    fn default() -> Self {
        ShareID::Flat(0u32)
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The struct to track the `ShareId`s and `ProposalIndex` associated with an organization
/// TODO: in the future, each of these should be separate maps
pub struct Organization<Hash> {
    /// The supervising ShareId for the organization, like a Board of Directors
    admin_id: ShareID,
    /// The constitution
    constitution: Hash,
}

impl<Hash: Clone> Organization<Hash> {
    pub fn new(admin_id: ShareID, constitution: Hash) -> Self {
        Organization {
            admin_id,
            constitution,
        }
    }
    pub fn admin_id(&self) -> ShareID {
        self.admin_id
    }
    pub fn constitution(&self) -> Hash {
        self.constitution.clone()
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// These are the types of formed and registered organizations in the `bank` module
pub enum FormedOrganization {
    FlatOrg(u32),
    FlatShares(u32, u32),
    WeightedShares(u32, u32),
}

impl From<u32> for FormedOrganization {
    fn from(other: u32) -> FormedOrganization {
        FormedOrganization::FlatOrg(other)
    }
}

impl From<(u32, ShareID)> for FormedOrganization {
    fn from(other: (u32, ShareID)) -> FormedOrganization {
        match other.1 {
            ShareID::Flat(share_id) => FormedOrganization::FlatShares(other.0, share_id),
            ShareID::WeightedAtomic(share_id) => {
                FormedOrganization::WeightedShares(other.0, share_id)
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The pieces of information used to register an organization in `bank`
pub enum OrganizationSource<AccountId, Shares> {
    /// Will be initialized as an organization with a single ShareId and equal governance strength from all members
    Accounts(Vec<AccountId>),
    /// "" weighted governance strength by Shares
    AccountsWeighted(Vec<(AccountId, Shares)>),
    /// References a share group registering to become an organization (OrgId, ShareId)
    SpinOffShareGroup(u32, ShareID),
}

impl<AccountId, Shares> Default for OrganizationSource<AccountId, Shares> {
    fn default() -> OrganizationSource<AccountId, Shares> {
        OrganizationSource::Accounts(Vec::new())
    }
}
