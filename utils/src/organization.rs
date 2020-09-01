use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub struct Relation<OrgId> {
    pub parent: OrgId,
    pub child: OrgId,
}

#[derive(new, PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
/// Used in `vote` and `donate` to distinguish between configurations that acknowledge ownership and don't
pub enum OrgRep<OrgId> {
    // weighted by ownership
    Weighted(OrgId),
    // equal for all members
    Equal(OrgId),
}

impl<OrgId: Copy> OrgRep<OrgId> {
    pub fn org(&self) -> OrgId {
        match self {
            OrgRep::Weighted(o) => *o,
            OrgRep::Equal(o) => *o,
        }
    }
}

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// Tracks main organization state
pub struct Organization<AccountId, OrgId, Shares, IpfsRef> {
    /// Optional sudo, encouraged to be None
    sudo: Option<AccountId>,
    /// Organization identifier
    id: OrgId,
    /// Total number of Shares
    shares: Shares,
    /// The constitution
    constitution: IpfsRef,
}

impl<
        AccountId: Clone + PartialEq,
        OrgId: Copy,
        Shares: Copy
            + sp_std::ops::Add<Output = Shares>
            + sp_std::ops::Sub<Output = Shares>,
        IpfsRef: Clone,
    > Organization<AccountId, OrgId, Shares, IpfsRef>
{
    pub fn id(&self) -> OrgId {
        self.id
    }
    pub fn constitution(&self) -> IpfsRef {
        self.constitution.clone()
    }
    pub fn total_shares(&self) -> Shares {
        self.shares
    }
    pub fn set_shares(&self, a: Shares) -> Self {
        Self {
            shares: a,
            ..self.clone()
        }
    }
    pub fn add_shares(&self, a: Shares) -> Self {
        Self {
            shares: self.shares + a,
            ..self.clone()
        }
    }
    pub fn subtract_shares(&self, a: Shares) -> Self {
        Self {
            shares: self.shares - a,
            ..self.clone()
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
            ..self.clone()
        }
    }
    pub fn put_sudo(&self, new_sudo: AccountId) -> Self {
        Organization {
            sudo: Some(new_sudo),
            ..self.clone()
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
}
