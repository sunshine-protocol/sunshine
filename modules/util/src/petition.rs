use crate::{
    traits::{Apply, Approved, Rejected, UpdatePetitionTerms, Vetoed},
    uuid::UUID4,
};
use codec::{Decode, Encode};
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The position of a voter in the petition
/// - this is used for getting acknowledgement for terms of agreement for example
pub enum PetitionView {
    /// Default no comment on the petition
    NoComment,
    /// Assent acknowledges the petition as legitimate and expresses support
    Assent,
    /// Dissent expresses against
    Dissent,
}

impl Default for PetitionView {
    fn default() -> PetitionView {
        PetitionView::NoComment
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct VetoContext<Hash> {
    // Reveals whether or not the veto has been invoked
    invoked: bool,
    // Reference to the requested changes information
    changes_requested: Option<Hash>,
    // Petition identifier for a petition which can revoke this veto if it passes
    // - usage equivalent to requesting explicit review by an outside group during the voting process
    revoke_if_vote_passes: Option<UUID4>,
}

impl<Hash: Clone + Default> VetoContext<Hash> {
    pub fn requested_changes(changes_requested: Hash) -> VetoContext<Hash> {
        VetoContext {
            invoked: true,
            changes_requested: Some(changes_requested),
            revoke_if_vote_passes: None,
        }
    }
    // made this a method for when this type becomes more complex and requires referencing the specific change made
    // - am aware that for now it is just the default but default it not invoked
    pub fn accept_changes() -> VetoContext<Hash> {
        VetoContext::default()
    }
    pub fn dispatched_external_review(review_id: UUID4) -> VetoContext<Hash> {
        VetoContext {
            invoked: true,
            changes_requested: None,
            revoke_if_vote_passes: Some(review_id),
        }
    }
    pub fn invoked(&self) -> bool {
        self.invoked
    }
    pub fn changes_requested(&self) -> Option<Hash> {
        self.changes_requested.clone()
    }
    pub fn revoke_if_vote_passes(&self) -> Option<UUID4> {
        self.revoke_if_vote_passes
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// Stored in 1 of 2 maps for `EndorsementHistory` or `VetoHistory`
/// TODO: decide the structure for the authenticated justification for `Veto`, `Assent`, etc
/// - (1) how are we authenticating strings?
/// - (2) how do we authenticate references to larger pieces of data stored elsewhere like IPFS?
pub struct PetitionSignature<AccountId, Hash> {
    signer: AccountId,
    view: PetitionView,
    justification: Hash,
}

impl<AccountId, Hash> PetitionSignature<AccountId, Hash> {
    pub fn new(signer: AccountId, view: PetitionView, justification: Hash) -> Self {
        PetitionSignature {
            signer,
            view,
            justification,
        }
    }
    pub fn view(&self) -> PetitionView {
        self.view
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The position of a voter in the petition
/// - this is used for getting acknowledgement for terms of agreement for example
pub enum PetitionOutcome {
    /// Waiting on some condition to be met before it is approved
    VoteWithNoOutcomeYet,
    /// Could be rejected or vetoed, waiting for time to expire but approved
    ApprovedButWaiting,
    /// Could pass, waiting for time to expire but rejected
    RejectedButWaiting,
    /// Frozen by vetoers, waiting for an update
    FrozenByVeto,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
}

impl Default for PetitionOutcome {
    fn default() -> PetitionOutcome {
        PetitionOutcome::VoteWithNoOutcomeYet
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
    /// Increment every time the topic hash changes and clear the `m` of the threshold
    version: u32,
    /// Frozen can only be unfrozen by an update or by vetoers that revoke their veto
    frozen: bool,
    /// Number of signers that signed in favor
    current_support: u32,
    /// Number of signers that need to sign for it to pass
    required_support: u32,
    /// Number of signers that signed against
    current_against: u32,
    /// Number of signers that need to sign for it to be rejected
    required_against: Option<u32>,
    /// Number of all members that can participate
    total_electorate: u32,
    /// Optional ending time
    ends: Option<BlockNumber>,
}
impl<Hash: Clone, BlockNumber: Clone> PetitionState<Hash, BlockNumber> {
    // TODO: break this into the valid object creation paths
    pub fn new(
        topic: Hash,
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
                version: 1u32, // notably starts at version 1u32
                frozen: false,
                current_support: 0u32,
                required_support,
                current_against: 0u32,
                required_against,
                total_electorate,
                ends,
            })
        } else {
            // does not satisfy the constraints for object creation
            None
        }
    }
    pub fn topic(&self) -> Hash {
        self.topic.clone()
    }
    pub fn version(&self) -> u32 {
        self.version
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
    pub fn total_electorate(&self) -> u32 {
        self.total_electorate
    }
    pub fn ends(&self) -> Option<BlockNumber> {
        self.ends.clone()
    }
    pub fn add_assent(&self) -> Self {
        let new_support = self.current_support() + 1u32;
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: self.frozen(),
            current_support: new_support,
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn revoke_assent(&self) -> Self {
        let new_support = self.current_support() - 1u32;
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: self.frozen(),
            current_support: new_support,
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn add_dissent(&self) -> Self {
        let new_against = self.current_against() + 1u32;
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: self.frozen(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: new_against,
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn revoke_dissent(&self) -> Self {
        let new_against = self.current_against() - 1u32;
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: self.frozen(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: new_against,
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn veto_to_freeze(&self) -> Self {
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: true,
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn revoke_veto_to_unfreeze(&self) -> Self {
        PetitionState {
            topic: self.topic(),
            version: self.version(),
            frozen: false,
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn update_without_clearing_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: new_topic,
            version: self.version() + 1u32,
            frozen: self.frozen(),
            current_support: self.current_support(),
            required_support: self.required_support(),
            current_against: self.current_against(),
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
    pub fn update_and_clear_petition_state(&self, new_topic: Hash) -> Self {
        PetitionState {
            topic: new_topic,
            version: self.version() + 1u32,
            frozen: false,
            current_support: 0u32,
            required_support: self.required_support(),
            current_against: 0u32,
            required_against: self.required_against(),
            total_electorate: self.total_electorate(),
            ends: self.ends(),
        }
    }
}
impl<Hash: Clone, BlockNumber: Clone> Apply<PetitionView> for PetitionState<Hash, BlockNumber> {
    // must check to see if the voter has already voted before applying new votes
    fn apply(&self, vote: PetitionView) -> PetitionState<Hash, BlockNumber> {
        match vote {
            PetitionView::Assent => self.add_assent(),
            // TODO: vetos are handled elsewhere
            PetitionView::Dissent => self.add_dissent(),
            // No Comment, odd to be applied
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
        self.current_support >= self.required_support
    }
}
impl<Hash: Clone, BlockNumber: Clone> Rejected for PetitionState<Hash, BlockNumber> {
    fn rejected(&self) -> bool {
        if let Some(req_against) = self.required_against {
            return self.current_against >= req_against;
        }
        false
    }
}
impl<Hash: Clone, BlockNumber: Clone> Vetoed for PetitionState<Hash, BlockNumber> {
    fn vetoed(&self) -> bool {
        // the existence of `frozen` might make `Vetoed` unnecessary but I want to keep it in case we add more constraints
        // that need to be checked in the context of the module
        self.frozen()
    }
}
