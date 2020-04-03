use crate::proposal::ProposalIndex;
use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
pub struct Organization<ShareId> {
    /// The shares registered by this organization
    shares: Vec<ShareId>,
    /// The proposals that are under consideration for this organization
    proposals: Vec<ProposalIndex>,
} // could add more share group distinctions

impl<ShareId: Parameter> Organization<ShareId> {
    pub fn new(admin_share_id: ShareId) -> Self {
        let mut shares = Vec::<ShareId>::new();
        shares.push(admin_share_id);
        Organization {
            shares,
            proposals: Vec::new(),
        }
    }

    /// Consumes the existing organization and outputs a new organization that
    /// includes the new share group identifier
    pub fn add_new_share_group(self, new_share_id: ShareId) -> Self {
        // could do this more efficiently
        let mut new_shares = self.clone().shares;
        new_shares.push(new_share_id);
        Organization {
            shares: new_shares,
            ..self
        }
    }

    pub fn add_proposal_index(self, proposal_index: ProposalIndex) -> Self {
        let mut new_proposals = self.clone().proposals;
        new_proposals.push(proposal_index);
        Organization {
            proposals: new_proposals,
            ..self
        }
    }
}
