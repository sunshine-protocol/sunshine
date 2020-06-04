use crate::share::ShareID;
use codec::{Codec, Decode, Encode};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The struct to track the organization's state
pub struct Organization<AccountId, Id: Codec + PartialEq + Zero + From<u32> + Copy, Hash> {
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

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Static terms of agreement, define how the enforced payout structure for grants
pub struct TermsOfAgreement<AccountId, Shares> {
    /// If Some(account), then account is the sudo for the duration of the grant
    supervisor: Option<AccountId>,
    /// The share allocation for metadata
    share_metadata: Vec<(AccountId, Shares)>,
}

impl<AccountId: Clone, Shares: Clone> TermsOfAgreement<AccountId, Shares> {
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
pub struct FullTermsOfAgreement<AccountId, Shares, OrgId, ShareId, BlockNumber> {
    /// The starting state for the group
    basic_terms: TermsOfAgreement<AccountId, Shares>,
    /// This represents the metagovernance configuration, how the group can coordinate changes
    allowed_changes: Vec<(
        Catalyst<AccountId>,
        Option<VoteConfig<AccountId, OrgId, ShareId, BlockNumber>>,
        Option<EnforcedOutcome<AccountId>>,
    )>,
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
pub enum VoteConfig<AccountId, OrgId, ShareId, BlockNumber> {
    /// Only one supervisor approval is required but everyone has veto rights, for BlockNumber after approval
    OneSupervisorApprovalWithFullOrgShareVetoRights(AccountId, OrgId, ShareId, BlockNumber),
    /// Two supervisor approvals is required but everyone has veto rights, for BlockNumber after approval
    TwoSupervisorsApprovalWithFullOrgShareVetoRights(
        AccountId,
        AccountId,
        OrgId,
        ShareId,
        BlockNumber,
    ),
    /// Only one supervisor approval is required but everyone can vote to veto must reach threshold, for BlockNumber after approval
    OneSupervisorApprovalWith1P1VCountThresholdVetoRights(
        AccountId,
        OrgId,
        ShareId,
        u32,
        BlockNumber,
    ),
    /// Two supervisor approvals is required but everyone can vote to veto must reach threshold, for BlockNumber after approval
    TwoSupervisorsApprovalWith1P1VCountThresholdVetoRights(
        AccountId,
        AccountId,
        OrgId,
        ShareId,
        u32,
        BlockNumber,
    ),
    /// Only one supervisor approval is required but everyone can vote to veto must reach share weighted threshold, for BlockNumber after approval
    OneSupervisorApprovalWithShareWeightedVetoRights(AccountId, OrgId, ShareId, u32, BlockNumber),
    /// Two supervisor approvals is required but everyone can vote to veto must reach share weighted threshold, for BlockNumber after approval
    TwoSupervisorsApprovalWithShareWeightedVetoRights(
        AccountId,
        AccountId,
        OrgId,
        ShareId,
        u32,
        BlockNumber,
    ),
    /// Warning: Dictatorial and Centralized Governance, some say _practical_
    OnePersonOneVoteThresholdWithOneSupervisorVetoRights(
        OrgId,
        ShareId,
        u32,
        AccountId,
        BlockNumber,
    ),
    OnePersonOneVoteThresholdWithTwoSupervisorsVetoRights(
        OrgId,
        ShareId,
        u32,
        AccountId,
        AccountId,
        BlockNumber,
    ),
    ShareWeightedVoteThresholdWithOneSupervisorVetoRights(
        OrgId,
        ShareId,
        u32,
        AccountId,
        BlockNumber,
    ),
    ShareWeightedVoteThresholdWithTwoSupervisorsVetoRights(
        OrgId,
        ShareId,
        u32,
        AccountId,
        AccountId,
        BlockNumber,
    ),
    /// 1 person 1 vote, u32 threshold for approval, but everyone has veto rights, for BlockNumber after approval
    OnePersonOneVoteThresholdWithFullOrgShareVetoRights(OrgId, ShareId, u32, BlockNumber),
    /// 1 person 1 vote, u32 threshold; only the second share group has veto rights (also must be flat!), for BlockNumber after approval
    OnePersonOneVoteThresholdANDVetoEnabledGroup(OrgId, ShareId, u32, ShareId, BlockNumber),
    /// ShareWeighted vote, u32 threshold for approval, but everyone has veto rights, for BlockNumber after approval
    ShareWeightedVoteThresholdWithFullOrgShareVetoRights(OrgId, ShareId, u32, BlockNumber),
    /// ShareWeighted vote, u32 threshold for approval, but everyone has veto rights, for BlockNumber after approval
    ShareWeightedVoteThresholdANDVetoEnabledGroup(OrgId, ShareId, u32, ShareId, BlockNumber),
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

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// These are the types of formed and registered organizations in the `bank` module
pub enum FormedOrganization<
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
> {
    FlatOrg(OrgId),
    FlatShares(OrgId, ShareId),
    WeightedShares(OrgId, ShareId),
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    > Default for FormedOrganization<OrgId, ShareId>
{
    fn default() -> FormedOrganization<OrgId, ShareId> {
        // default org, might be endowed with _special_ power
        FormedOrganization::FlatOrg(OrgId::zero())
    }
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    > From<OrgId> for FormedOrganization<OrgId, ShareId>
{
    fn from(other: OrgId) -> FormedOrganization<OrgId, ShareId> {
        FormedOrganization::FlatOrg(other)
    }
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    > From<(OrgId, ShareID<ShareId>)> for FormedOrganization<OrgId, ShareId>
{
    fn from(other: (OrgId, ShareID<ShareId>)) -> FormedOrganization<OrgId, ShareId> {
        match other.1 {
            ShareID::Flat(share_id) => FormedOrganization::FlatShares(other.0, share_id),
            ShareID::Weighted(share_id) => FormedOrganization::WeightedShares(other.0, share_id),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
/// The pieces of information used to register an organization in `bank`
pub enum OrganizationSource<
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
    AccountId,
    Shares,
> {
    /// Will be initialized as an organization with a single ShareId and equal governance strength from all members
    Accounts(Vec<AccountId>),
    /// "" weighted governance strength by Shares
    AccountsWeighted(Vec<(AccountId, Shares)>),
    /// References a share group registering to become an organization (OrgId, ShareId)
    SpinOffShareGroup(OrgId, ShareID<ShareId>),
}

impl<
        OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
        ShareId: Codec + PartialEq + Zero + From<u32> + Copy,
        AccountId: PartialEq,
        Shares,
    > Default for OrganizationSource<OrgId, ShareId, AccountId, Shares>
{
    fn default() -> OrganizationSource<OrgId, ShareId, AccountId, Shares> {
        OrganizationSource::Accounts(Vec::new())
    }
}
