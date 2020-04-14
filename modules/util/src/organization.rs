use crate::bounty::{BountyId, MilestoneId};
use crate::proposal::ProposalIndex;
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Static terms of agreement, define how the enforced payout structure for grants
pub struct TermsOfAgreement<AccountId, Shares> {
    supervisor: Option<AccountId>,
    share_metadata: Vec<(AccountId, Shares)>,
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Defined paths for how the terms of agreement can change
pub struct FullTermsOfAgreement<OrgId, ShareId, AccountId, Shares> {
    /// The starting state for the group
    basic_terms: TermsOfAgreement<AccountId, Shares>,
    /// This represents the metagovernance configuration, how the group can coordinate changes
    allowed_changes: Vec<(
        Catalyst<AccountId>,
        Option<RequiredVote<OrgId, ShareId>>,
        Option<EnforcedOutcome<OrgId, ShareId, AccountId>>,
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
pub enum RequiredVote<OrgId, ShareId> {
    /// Only one supervisor approval is required but everyone has veto rights
    OneSupervisorApprovalWithVetoRights(OrgId, ShareId),
    /// Two supervisor approvals is required but everyone has veto rights
    TwoSupervisorsApprovalWithVetoRights(OrgId, ShareId),
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum EnforcedOutcome<OrgId, ShareId, AccountId> {
    /// Grant paid out as per bounty (hosting org, bounty recipient, milestone in question)
    GrantPayoutBasedOnShareDistribution(OrgId, ShareId, BountyId, MilestoneId),
    /// Remove member for unacceptable behavior
    RemoveMemberForBadBehavior(OrgId, ShareId, AccountId),
    /// Swap the first account for the second account in the same role for a grant team
    SwapRoleOnGrantTeam(OrgId, ShareId, AccountId, AccountId),
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The struct to track the `ShareId`s and `ProposalIndex` associated with an organization
pub struct Organization<ShareId> {
    /// The shares registered by this organization
    inner_shares: Vec<ShareId>,
    /// The suborganizations completing ongoing bounties
    funded_teams: Vec<ShareId>,
    /// The proposals that are under consideration for this organization
    /// TODO: consider adding share group context so that vote schedules are available to every share group to make decisions
    /// - the constraints on the votes that can be included in the vote schedule are derived from the actor
    /// and their relationships
    proposals: Vec<ProposalIndex>,
}

impl<ShareId: Parameter> Organization<ShareId> {
    pub fn new(admin_share_id: ShareId) -> Self {
        let mut inner_shares = Vec::<ShareId>::new();
        let funded_teams = inner_shares.clone();
        inner_shares.push(admin_share_id);
        Organization {
            inner_shares,
            funded_teams,
            proposals: Vec::new(),
        }
    }

    /// Consumes the existing organization and outputs a new organization that
    /// includes the new share group identifier
    /// TODO: split into two methods, one for bounty and one for governance
    pub fn add_new_inner_share_group(self, new_share_id: ShareId) -> Self {
        // could do this more efficiently
        let mut new_shares = self.clone().inner_shares;
        new_shares.push(new_share_id);
        Organization {
            inner_shares: new_shares,
            ..self
        }
    }

    /// Consumes the existing organization and outputs a new organization that
    /// includes the new proposal index
    pub fn add_proposal_index(self, proposal_index: ProposalIndex) -> Self {
        let mut new_proposals = self.clone().proposals;
        new_proposals.push(proposal_index);
        Organization {
            proposals: new_proposals,
            ..self
        }
    }
}
