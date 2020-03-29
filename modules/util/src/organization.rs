use crate::proposal::ProposalIndex;
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// TODO: add ModuleId builder
pub struct Organization<OrgId, ShareId> {
    /// The organizational identifier
    id: OrgId,
    /// The shares registered by this organization
    shares: Vec<ShareId>,
    /// The proposals that are under consideration for this organization
    proposals: Vec<ProposalIndex>,
}

impl<OrgId: Parameter, ShareId: Parameter> Organization<OrgId, ShareId> {
    pub fn new(id: OrgId, admin_share_id: ShareId) -> Self {
        let mut shares = Vec::<ShareId>::new();
        shares.push(admin_share_id);
        Organization {
            id,
            shares,
            proposals: Vec::new(),
        }
    }

    /// Consumes the existing organization and outputs a new organization that
    /// includes the new share group identifier
    pub fn add_new_share_group(self, new_share_id: ShareId) -> Self {
        let mut new_shares = self.clone().shares;
        new_shares.push(new_share_id);
        Organization {
            shares: new_shares,
            ..self
        }
    }
}
