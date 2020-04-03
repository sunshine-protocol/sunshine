use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct OrgSharePrefixKey<OrgId, ShareId> {
    org: OrgId,
    share: ShareId,
}

impl<OrgId: Parameter + Copy, ShareId: Parameter + Copy> OrgSharePrefixKey<OrgId, ShareId> {
    pub fn new(org: OrgId, share: ShareId) -> OrgSharePrefixKey<OrgId, ShareId> {
        OrgSharePrefixKey { org, share }
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn share(&self) -> ShareId {
        self.share
    }
}

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct OrgShareVotePrefixKey<OrgId, ShareId, VoteId> {
    org: OrgId,
    share: ShareId,
    vote: VoteId,
}

impl<OrgId: Parameter + Copy, ShareId: Parameter + Copy, VoteId: Parameter + Copy>
    OrgShareVotePrefixKey<OrgId, ShareId, VoteId>
{
    pub fn new(
        org: OrgId,
        share: ShareId,
        vote: VoteId,
    ) -> OrgShareVotePrefixKey<OrgId, ShareId, VoteId> {
        OrgShareVotePrefixKey { org, share, vote }
    }
    pub fn org_share_prefix(&self) -> OrgSharePrefixKey<OrgId, ShareId> {
        OrgSharePrefixKey::new(self.org, self.share)
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn share(&self) -> ShareId {
        self.share
    }
    pub fn vote(&self) -> VoteId {
        self.vote
    }
}
