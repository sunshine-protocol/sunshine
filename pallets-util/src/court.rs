use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
pub struct Evidence<AccountId, Hash> {
    poster: AccountId,
    evidence: Hash,
    acknowledge_by_all_parties: bool,
}

impl<AccountId, Hash> Evidence<AccountId, Hash> {
    pub fn new(poster: AccountId, evidence: Hash) -> Evidence<AccountId, Hash> {
        Evidence {
            poster,
            evidence,
            acknowledge_by_all_parties: false,
        }
    }
    pub fn was_acknowledged_by_all_parties(&self) -> bool {
        self.acknowledge_by_all_parties
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ExternalDisputeTypes<AccountId> {
    /// Invoked by a reviewer during review
    Veto(AccountId),
    /// Contract cancelled
    ContractCancelled,
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum MilestoneTeamResponse<Hash> {
    /// The team is claiming that the new deliverable meets the feedback that resulted in the dispute being raised
    UpdateSubmissionForFeedback(Hash),
    /// Propose a milestone adjustment which can be approved but is currently initiated by
    /// the supervisor's supervisor (which should be governance eventually but is a sudo key now)
    ProposeMilestoneAdjustment(Hash),
    /// Despite the supervisor not approving the deliverable, the team is claiming they delivered
    /// - a chain explorer might collect all complaints for each organization and post that metadata in the
    /// UI for the freelance developers
    ClaimDeliverableMeetsRequirements,
}
