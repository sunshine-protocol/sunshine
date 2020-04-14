use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Evidence<Hash> {
    /// The hash chain (see adapter::HashChain)
    evidence: Vec<Hash>,
    /// Reset every time a hash is added to the hash chain
    acknowledge_by_all_parties: bool,
}

impl<Hash> Default for Evidence<Hash> {
    fn default() -> Evidence<Hash> {
        Evidence {
            evidence: Vec::<Hash>::new(),
            acknowledge_by_all_parties: true,
        }
    }
}

impl<Hash> Evidence<Hash> {
    // fn new_with_history(evidence: Vec<Hash>) -> Evidence<Hash> {
    //     Evidence {
    //         evidence,
    //         acknowledge_by_all_parties: false,
    //     }
    // }
    pub fn check_acknowledged_by_all_parties(&self) -> bool {
        self.acknowledge_by_all_parties
    }

    /// Must be called after any change the evidence chain until acknowledgements are met
    pub fn set_not_acknowledged_by_all_parties(&mut self) {
        self.acknowledge_by_all_parties = false;
    }

    /// Can only be called in a specific context in which all required acknowledgers have acknowledged
    pub fn set_acknowledged_by_all_parties(&mut self) {
        self.acknowledge_by_all_parties = true;
    }

    // TODO: move these into an `Append<Hash>` trait bound on this object
    pub fn append(&mut self, hash: Hash) {
        self.evidence.push(hash);
        self.set_not_acknowledged_by_all_parties();
    }
    pub fn append_vec(&mut self, mut hashes: Vec<Hash>) {
        self.evidence.append(&mut hashes);
        self.set_not_acknowledged_by_all_parties();
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
