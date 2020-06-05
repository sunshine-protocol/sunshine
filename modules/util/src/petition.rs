use crate::traits::{Apply, Approved, Rejected, UpdatePetitionTerms};
use codec::{Decode, Encode};
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The position of a voter in the petition
/// - this is used for getting acknowledgement for terms of agreement for example
pub enum PetitionView<Hash> {
    /// Assent acknowledges the petition as legitimate and expresses support
    Assent(Hash),
    /// Veto the given thing with a reason
    Veto(Hash),
    /// Default no comment on the petition but shows up in turnout?
    NoComment,
}

impl<Hash> PetitionView<Hash> {
    pub fn ipfs_reference(self) -> Option<Hash> {
        match self {
            PetitionView::Assent(cid) => Some(cid),
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

#[derive(new, Default, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct PetitionSignature<AccountId, Hash> {
    signer: AccountId,
    view: PetitionView<Hash>,
}

impl<AccountId, Hash: Clone> PetitionSignature<AccountId, Hash> {
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
    /// Approved
    Approved,
    /// Rejected
    Rejected,
}

impl Approved for PetitionOutcome {
    fn approved(&self) -> bool {
        match self {
            PetitionOutcome::Approved => true,
            _ => false,
        }
    }
}

impl Default for PetitionOutcome {
    fn default() -> PetitionOutcome {
        PetitionOutcome::VotingWithNoOutcomeYet
    }
}

#[derive(Clone, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The state of a petition at any given time
pub struct PetitionState<OrgId, Hash, BlockNumber>
where
    Hash: Clone,
{
    /// The topic corresponds to some authentication used to identify _what_ is voted on
    topic: Option<Hash>,
    /// Vote group identifier
    voter_group: OrgId,
    /// Number of signers that signed in favor
    current_support: u32,
    /// Number of signers that need to sign for it to pass
    required_support: u32,
    /// Number of open vetos
    veto_count: u32,
    /// Number of vetos required to freeze the vote
    vetos_to_reject: u32,
    /// Number of all members that can participate
    total_electorate: u32,
    /// The petition's outcome
    outcome: PetitionOutcome,
    /// Optional ending time
    ends: Option<BlockNumber>,
}
impl<OrgId: Copy, Hash: Clone, BlockNumber: Clone> PetitionState<OrgId, Hash, BlockNumber> {
    pub fn new(
        topic: Option<Hash>,
        voter_group: OrgId,
        required_support: u32,
        vetos_to_reject: u32,
        total_electorate: u32,
        ends: Option<BlockNumber>,
    ) -> Option<PetitionState<OrgId, Hash, BlockNumber>> {
        let constraints: bool =
            total_electorate >= required_support && total_electorate >= vetos_to_reject;
        if constraints {
            Some(PetitionState {
                topic,
                voter_group,
                current_support: 0u32,
                required_support,
                veto_count: 0u32,
                vetos_to_reject,
                total_electorate,
                outcome: PetitionOutcome::VotingWithNoOutcomeYet,
                ends,
            })
        } else {
            // does not satisfy the constraints for object creation
            None
        }
    }
    pub fn voter_group(&self) -> OrgId {
        self.voter_group
    }
    pub fn topic(&self) -> Option<Hash> {
        self.topic.clone()
    }
    pub fn current_support(&self) -> u32 {
        self.current_support
    }
    pub fn required_support(&self) -> u32 {
        self.required_support
    }
    pub fn veto_count(&self) -> u32 {
        self.veto_count
    }
    pub fn vetos_to_reject(&self) -> u32 {
        self.vetos_to_reject
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
            voter_group: self.voter_group,
            current_support: new_support,
            required_support: self.required_support(),
            veto_count: self.veto_count(),
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn revoke_assent(&self) -> Self {
        let new_support = self.current_support() - 1u32;
        PetitionState {
            topic: self.topic(),
            voter_group: self.voter_group,
            current_support: new_support,
            required_support: self.required_support(),
            veto_count: self.veto_count(),
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn add_veto(&self) -> Self {
        let new_veto_count = self.veto_count() + 1u32;
        PetitionState {
            topic: self.topic(),
            voter_group: self.voter_group,
            current_support: self.current_support(),
            required_support: self.required_support(),
            veto_count: new_veto_count,
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn revoke_veto(&self) -> Self {
        let new_veto_count = self.veto_count - 1u32;
        PetitionState {
            topic: self.topic(),
            voter_group: self.voter_group,
            current_support: self.current_support(),
            required_support: self.required_support(),
            veto_count: new_veto_count,
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn update_without_clearing_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: Some(new_topic),
            voter_group: self.voter_group,
            current_support: self.current_support(),
            required_support: self.required_support(),
            veto_count: self.veto_count(),
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    pub fn update_and_clear_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: Some(new_topic),
            voter_group: self.voter_group,
            current_support: 0u32,
            required_support: self.required_support(),
            veto_count: 0u32,
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: self.outcome(),
            ends: self.ends(),
        }
    }
    // dangerous, no checks on this before the SET
    pub fn set_outcome(&self, new_outcome: PetitionOutcome) -> Self {
        PetitionState {
            topic: self.topic(),
            voter_group: self.voter_group,
            current_support: self.current_support(),
            required_support: self.required_support(),
            veto_count: self.veto_count(),
            vetos_to_reject: self.vetos_to_reject(),
            total_electorate: self.total_electorate(),
            outcome: new_outcome,
            ends: self.ends(),
        }
    }
}
impl<OrgId: Copy, Hash: Clone, BlockNumber: Clone> Apply<PetitionView<Hash>>
    for PetitionState<OrgId, Hash, BlockNumber>
{
    // NOTE: must check to see if the voter has already voted before applying new votes
    fn apply(&self, vote: PetitionView<Hash>) -> PetitionState<OrgId, Hash, BlockNumber> {
        match vote {
            PetitionView::Assent(_) => self.add_assent(),
            PetitionView::Veto(_) => self.add_veto(),
            // No comment, nothing applied
            _ => self.clone(),
        }
    }
}
impl<OrgId: Copy, Hash: Clone, BlockNumber: Clone> UpdatePetitionTerms<Hash>
    for PetitionState<OrgId, Hash, BlockNumber>
{
    fn update_petition_terms(&self, new_terms: Hash, clear_votes_on_update: bool) -> Self {
        if clear_votes_on_update {
            self.update_and_clear_petition_state(new_terms)
        } else {
            self.update_without_clearing_petition_state(new_terms)
        }
    }
}
impl<OrgId: Copy, Hash: Clone, BlockNumber: Clone> Approved
    for PetitionState<OrgId, Hash, BlockNumber>
{
    fn approved(&self) -> bool {
        (self.veto_count() < self.vetos_to_reject())
            && (self.current_support() >= self.required_support())
    }
}
impl<OrgId: Copy, Hash: Clone, BlockNumber: Clone> Rejected
    for PetitionState<OrgId, Hash, BlockNumber>
{
    fn rejected(&self) -> bool {
        self.veto_count() >= self.vetos_to_reject()
    }
}
