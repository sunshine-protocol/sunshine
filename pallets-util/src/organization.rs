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

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The struct to track the organization's state
pub struct Organization<
    AccountId,
    Id: Codec + PartialEq + Zero + From<u32> + Copy,
    Hash,
> {
    /// The default sudo for this organization, optional because not _encouraged_
    sudo: Option<AccountId>,
    /// The parent organization for this organization
    parent_id: Option<Id>,
    /// The constitution
    constitution: Hash,
}

impl<
        AccountId: Clone + PartialEq,
        Id: Codec + PartialEq + Zero + From<u32> + Copy,
        Hash: Clone,
    > Organization<AccountId, Id, Hash>
{
    pub fn parent(&self) -> Option<Id> {
        self.parent_id
    }
    pub fn constitution(&self) -> Hash {
        self.constitution.clone()
    }
    pub fn is_parent(&self, cmp: Id) -> bool {
        if let Some(unwrapped_parent) = self.parent_id {
            unwrapped_parent == cmp
        } else {
            false
        }
    }
    pub fn is_sudo(&self, cmp: &AccountId) -> bool {
        if let Some(unwrapped_sudo) = &self.sudo {
            unwrapped_sudo == cmp
        } else {
            false
        }
    }
    pub fn clear_sudo(&self) -> Self {
        Organization {
            sudo: None,
            parent_id: self.parent_id,
            constitution: self.constitution.clone(),
        }
    }
    pub fn put_sudo(&self, new_sudo: AccountId) -> Self {
        Organization {
            sudo: Some(new_sudo),
            parent_id: self.parent_id,
            constitution: self.constitution.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The pieces of information used to register an organization in `org`
pub enum OrganizationSource<AccountId, Shares> {
    /// Will be initialized as an organization with a single ShareId and equal governance strength from all members
    Accounts(Vec<AccountId>),
    /// "" weighted governance strength by Shares
    AccountsWeighted(Vec<(AccountId, Shares)>),
}
impl<AccountId: PartialEq, Shares> From<Vec<(AccountId, Shares)>>
    for OrganizationSource<AccountId, Shares>
{
    fn from(
        other: Vec<(AccountId, Shares)>,
    ) -> OrganizationSource<AccountId, Shares> {
        OrganizationSource::AccountsWeighted(other)
    }
}
impl<AccountId: PartialEq, Shares> Default
    for OrganizationSource<AccountId, Shares>
{
    fn default() -> OrganizationSource<AccountId, Shares> {
        OrganizationSource::Accounts(Vec::new())
    }
}

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Static terms of agreement, define how the enforced payout structure for grants
pub struct TermsOfAgreement<AccountId, Shares, Hash> {
    /// Value constitution
    constitution: Hash,
    /// If Some(account), then account is the sudo for the duration of the grant
    supervisor: Option<AccountId>,
    /// The share allocation for metadata
    share_metadata: Vec<(AccountId, Shares)>,
}

impl<AccountId: Clone, Shares: Clone, Hash: Clone>
    TermsOfAgreement<AccountId, Shares, Hash>
{
    pub fn constitution(&self) -> Hash {
        self.constitution.clone()
    }
    pub fn supervisor(&self) -> Option<AccountId> {
        self.supervisor.clone()
    }
    pub fn flat(&self) -> Vec<AccountId> {
        self.share_metadata
            .clone()
            .into_iter()
            .map(|(account, _)| account)
            .collect::<Vec<AccountId>>()
    }
    pub fn weighted(&self) -> Vec<(AccountId, Shares)> {
        self.share_metadata.clone()
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Defined paths for how the terms of agreement can change
pub struct FullTermsOfAgreement<AccountId, Rules, Decisions, Outcomes> {
    /// The starting state for the group
    basic_terms: Rules,
    /// This represents the metagovernance configuration, how the group can coordinate changes
    allowed_changes:
        Vec<(Catalyst<AccountId>, Option<Decisions>, Option<Outcomes>)>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// Authenticates that the given user can do the action in question to
/// trigger the `VoteConfig`
pub enum Catalyst<AccountId> {
    ReportBadBehavior(AccountId),
    SubmitMilestone(AccountId),
    RequestMilestoneAdjustment(AccountId),
    SwapRole(AccountId, AccountId),
} // TODO: upgrade path from suborganization to separate organization

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// These are all the vote configs planned to be supported
/// TODO: we have n = 1, 2 so do it with AccountId, up to `12`, one day somehow
pub enum VoteConfig<AccountId, OrgId, BlockNumber> {
    /// Only one supervisor approval is required but everyone has veto rights, for BlockNumber after approval
    OneSupervisorApprovalWithFullOrgShareVetoRights(
        AccountId,
        OrgId,
        BlockNumber,
    ),
    /// Two supervisor approvals is required but everyone has veto rights, for BlockNumber after approval
    TwoSupervisorsApprovalWithFullOrgShareVetoRights(
        AccountId,
        AccountId,
        OrgId,
        BlockNumber,
    ),
    /// Only one supervisor approval is required but everyone can vote to veto must reach threshold, for BlockNumber after approval
    OneSupervisorApprovalWith1P1VCountThresholdVetoRights(
        AccountId,
        OrgId,
        u32,
        BlockNumber,
    ),
    /// Two supervisor approvals is required but everyone can vote to veto must reach threshold, for BlockNumber after approval
    TwoSupervisorsApprovalWith1P1VCountThresholdVetoRights(
        AccountId,
        AccountId,
        OrgId,
        u32,
        BlockNumber,
    ),
    /// Only one supervisor approval is required but everyone can vote to veto must reach share weighted threshold, for BlockNumber after approval
    OneSupervisorApprovalWithShareWeightedVetoRights(
        AccountId,
        OrgId,
        u32,
        BlockNumber,
    ),
    /// Two supervisor approvals is required but everyone can vote to veto must reach share weighted threshold, for BlockNumber after approval
    TwoSupervisorsApprovalWithShareWeightedVetoRights(
        AccountId,
        AccountId,
        OrgId,
        u32,
        BlockNumber,
    ),
    /// Warning: Dictatorial and Centralized Governance, some say _practical_
    OnePersonOneVoteThresholdWithOneSupervisorVetoRights(
        OrgId,
        u32,
        AccountId,
        BlockNumber,
    ),
    OnePersonOneVoteThresholdWithTwoSupervisorsVetoRights(
        OrgId,
        u32,
        AccountId,
        AccountId,
        BlockNumber,
    ),
    ShareWeightedVoteThresholdWithOneSupervisorVetoRights(
        OrgId,
        u32,
        AccountId,
        BlockNumber,
    ),
    ShareWeightedVoteThresholdWithTwoSupervisorsVetoRights(
        OrgId,
        u32,
        AccountId,
        AccountId,
        BlockNumber,
    ),
    /// 1 person 1 vote, u32 threshold for approval, but everyone has veto rights, for BlockNumber after approval
    OnePersonOneVoteThresholdWithFullOrgShareVetoRights(
        OrgId,
        u32,
        BlockNumber,
    ),
    /// 1 person 1 vote, u32 threshold; only the second share group has veto rights (also must be flat!), for BlockNumber after approval
    OnePersonOneVoteThresholdANDVetoEnabledGroup(
        OrgId,
        u32,
        OrgId,
        BlockNumber,
    ),
    /// ShareWeighted vote, u32 threshold for approval, but everyone has veto rights, for BlockNumber after approval
    ShareWeightedVoteThresholdWithFullOrgShareVetoRights(
        OrgId,
        u32,
        BlockNumber,
    ),
    /// ShareWeighted vote, u32 threshold for approval, but everyone in second org has veto rights, for BlockNumber after approval
    ShareWeightedVoteThresholdANDVetoEnabledGroup(
        OrgId,
        u32,
        OrgId,
        BlockNumber,
    ),
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
