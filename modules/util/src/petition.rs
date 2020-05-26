use crate::traits::{Apply, Approved, Rejected, UpdatePetitionTerms};
use codec::{Decode, Encode};
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The position of a voter in the petition
/// - this is used for getting acknowledgement for terms of agreement for example
pub enum PetitionView<Hash> {
    /// Assent acknowledges the petition as legitimate and expresses support
    Assent(Hash),
    /// Dissent expresses against
    Dissent(Hash),
    /// Veto the given thing with a reason
    Veto(Hash),
    /// Default no comment on the petition but shows up in turnout?
    NoComment,
}

impl<Hash> PetitionView<Hash> {
    pub fn ipfs_reference(self) -> Option<Hash> {
        match self {
            PetitionView::Assent(cid) => Some(cid),
            PetitionView::Dissent(cid) => Some(cid),
            PetitionView::Veto(cid) => Some(cid),
            _ => None,
        }
    }
}

impl<Hash> Default for PetitionView<Hash> {
    fn default() -> PetitionView<Hash> {
        PetitionView::NoComment
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct PetitionSignature<AccountId, Hash> {
    signer: AccountId,
    view: PetitionView<Hash>,
}

impl<AccountId, Hash: Clone> PetitionSignature<AccountId, Hash> {
    pub fn new(signer: AccountId, view: PetitionView<Hash>) -> Self {
        PetitionSignature { signer, view }
    }
    pub fn view(&self) -> PetitionView<Hash> {
        self.view.clone()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The position of a voter in the petition
/// - this is used for getting acknowledgement for terms of agreement for example
pub enum PetitionOutcome {
    /// Waiting on some condition to be met before it is approved
    VotingWithNoOutcomeYet,
    /// Could be rejected or vetoed, waiting for time to expire but approved
    ApprovedButWaitingForTimeToExpire,
    /// Could pass, waiting for time to expire but rejected
    RejectedButWaitingForTimeToExpire,
    /// Frozen by vetoers, waiting for an update
    FrozenByVetoButWaitingForRequestChanges,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
}

impl Default for PetitionOutcome {
    fn default() -> PetitionOutcome {
        PetitionOutcome::VotingWithNoOutcomeYet
    }
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of a petition at any given time
pub struct PetitionState<Hash, BlockNumber>
where
    Hash: Clone,
{
    /// The topic corresponds to some authentication used to identify _what_ is voted on
    topic: Hash,
    /// Frozen can only be unfrozen by an update or by vetoers that revoke their veto
    frozen: bool,
    /// Vote qualifier that is OrgId, FlatShareId
    voter_id_reqs: (u32, u32),
    /// Number of signers that signed in favor
    current_support: u32,
    /// Number of signers that need to sign for it to pass
    required_support: u32,
    /// Number of signers that signed against
    current_against: u32,
    /// Number of signers that need to sign for it to be formally rejected
    required_against: Option<u32>,
    /// Number of open vetos
    veto_count: u32,
    /// Number of all members that can participate
    total_electorate: u32,
    /// The petition's outcome
    outcome: PetitionOutcome,
    /// Optional ending time
    ends: Option<BlockNumber>,
}
impl<Hash: Clone, BlockNumber: Clone> PetitionState<Hash, BlockNumber> {
    // TODO: break this into the valid object creation paths
    pub fn new(
        topic: Hash,
        voter_id_reqs: (u32, u32),
        required_support: u32,
        required_against: Option<u32>,
        total_electorate: u32,
        ends: Option<BlockNumber>,
    ) -> Option<PetitionState<Hash, BlockNumber>> {
        let constraints: bool = total_electorate >= required_support
            && if let Some(req_against) = required_against {
                total_electorate >= req_against
            } else {
                true
            };
        if constraints {
            Some(PetitionState {
                topic,
                frozen: false,
                voter_id_reqs,
                current_support: 0u32,
                required_support,
                current_against: 0u32,
                required_against,
                veto_count: 0u32,
                total_electorate,
                outcome: PetitionOutcome::VotingWithNoOutcomeYet,
                ends,
            })
        } else {
            // does not satisfy the constraints for object creation
            None
        }
    }
    pub fn voter_id_reqs(&self) -> (u32, u32) {
        self.voter_id_reqs
    }
    pub fn topic(&self) -> Hash {
        self.topic.clone()
    }
    pub fn frozen(&self) -> bool {
        self.frozen
    }
    pub fn current_support(&self) -> u32 {
        self.current_support
    }
    pub fn required_support(&self) -> u32 {
        self.required_support
    }
    pub fn current_against(&self) -> u32 {
        self.current_against
    }
    pub fn required_against(&self) -> Option<u32> {
        self.required_against
    }
    pub fn veto_count(&self) -> u32 {
        self.veto_count
    }
    pub fn total_electorate(&self) -> u32 {
        self.total_electorate
    }
    pub fn outcome(&self) -> PetitionOutcome {
        self.outcome
    }
    pub fn ends(&self) -> Option<BlockNumber> {
        self.ends.clone()
    }
    // NOTE: cannot adjust outcome because no context here for time until expiry
    pub fn add_assent(&self) -> Self {
        let new_support = self.current_support() + 1u32;
        PetitionState {
            topic: self.topic(),
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: new_support,
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn revoke_assent(&self) -> Self {
        let new_support = self.current_support() - 1u32;
        PetitionState {
            topic: self.topic(),
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: new_support,
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn add_dissent(&self) -> Self {
        let new_against = self.current_against() + 1u32;
        PetitionState {
            topic: self.topic(),
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: new_against,
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn revoke_dissent(&self) -> Self {
        let new_against = self.current_against() - 1u32;
        PetitionState {
            topic: self.topic(),
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: new_against,
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn add_veto(&self) -> Self {
        let new_veto_count = self.veto_count() + 1u32;
        PetitionState {
            topic: self.topic(),
            frozen: true, // freeze petition state
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: new_veto_count,
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn revoke_veto(&self) -> Self {
        let new_veto_count = self.veto_count - 1u32;
        let frozen = if new_veto_count == 0 { false } else { true };
        PetitionState {
            topic: self.topic(),
            frozen,
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: new_veto_count,
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn update_without_clearing_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: new_topic,
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn update_and_clear_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: new_topic,
            frozen: false,
            voter_id_reqs: self.voter_id_reqs(),
            current_support: 0u32,
            required_support: self.required_support(),
            current_against: 0u32,
            required_against: self.required_against(),
            veto_count: 0u32,
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    // dangerous, no checks on this before the SET
    pub fn set_outcome(&self, new_outcome: PetitionOutcome) -> Self {
        PetitionState {
            topic: self.topic(),
            frozen: self.frozen(),
            voter_id_reqs: self.voter_id_reqs(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            veto_count: self.veto_count(),
            total_electorate: self.total_electorate(),
            outcome: new_outcome,
            ends: self.ends(),
        }
    }
}
impl<Hash: Clone, BlockNumber: Clone> Apply<PetitionView<Hash>>
    for PetitionState<Hash, BlockNumber>
{
    // must check to see if the voter has already voted before applying new votes
    fn apply(&self, vote: PetitionView<Hash>) -> PetitionState<Hash, BlockNumber> {
        match vote {
            PetitionView::Assent(_) => self.add_assent(),
            PetitionView::Dissent(_) => self.add_dissent(),
            PetitionView::Veto(_) => self.add_veto(),
            // No comment, nothing applied
            _ => self.clone(),
        }
    }
}
impl<Hash: Clone, BlockNumber: Clone> UpdatePetitionTerms<Hash>
    for PetitionState<Hash, BlockNumber>
{
    /// Resets thresholds by default every time the petition's topic changes
    fn update_petition_terms(&self, new_terms: Hash) -> Self {
        // we choose here to enable updates without clearing the petition state but this assumption
        // isn't always correct
        // TODO: think about how to add more explicit configurability, rn it is just this line
        self.update_without_clearing_petition_state(new_terms)
    }
}

impl<Hash: Clone, BlockNumber: Clone> Approved for PetitionState<Hash, BlockNumber> {
    fn approved(&self) -> bool {
        !self.frozen() && (self.current_support() >= self.required_support())
    }
}
impl<Hash: Clone, BlockNumber: Clone> Rejected for PetitionState<Hash, BlockNumber> {
    fn rejected(&self) -> bool {
        if let Some(req_against) = self.required_against() {
            !self.frozen() && (self.current_against >= req_against)
        } else {
            false
        }
    }
}
