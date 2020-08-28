use codec::{
    Decode,
    Encode,
};
use orml_utilities::OrderedSet;
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
use sp_std::{
    cmp::{
        Eq,
        Ordering,
    },
    prelude::*,
};

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub enum DocState<VoteId, Id> {
    WaitingForApproval,
    Voting(VoteId),
    ApprovedAndAdded(Id),
}

#[derive(
    Clone, Copy, Eq, PartialEq, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct DocProposal<CommitteeId, ProposalId, Cid, AccountId, State> {
    id: (CommitteeId, ProposalId),
    doc: Cid,
    proposer: AccountId,
    state: State,
}

impl<
        CommitteeId: Copy,
        ProposalId: Copy,
        Cid: Clone,
        AccountId: Clone,
        VoteId: Copy,
        DocId: Copy,
    >
    DocProposal<
        CommitteeId,
        ProposalId,
        Cid,
        AccountId,
        DocState<VoteId, DocId>,
    >
{
    pub fn new(
        committee_id: CommitteeId,
        proposal_id: ProposalId,
        doc: Cid,
        proposer: AccountId,
    ) -> Self {
        Self {
            id: (committee_id, proposal_id),
            doc,
            proposer,
            state: DocState::WaitingForApproval,
        }
    }
    pub fn committee_id(&self) -> CommitteeId {
        self.id.0
    }
    pub fn proposal_id(&self) -> ProposalId {
        self.id.1
    }
    pub fn doc(&self) -> Cid {
        self.doc.clone()
    }
    pub fn proposer(&self) -> AccountId {
        self.proposer.clone()
    }
    pub fn state(&self) -> DocState<VoteId, DocId> {
        self.state
    }
    pub fn set_state(&self, s: DocState<VoteId, DocId>) -> Self {
        Self {
            state: s,
            ..self.clone()
        }
    }
}

#[derive(new, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
/// The shape of docs after added to RFC Committee (Doc generic param for RfcBoard)
pub struct DocRef<Id, Cid> {
    pub id: Id,
    pub doc: Cid,
}

impl<Id: Copy + Eq + Ord, Cid: Clone + Eq> Eq for DocRef<Id, Cid> {}

impl<Id: Copy + Eq + Ord, Cid: Clone + Eq> Ord for DocRef<Id, Cid> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl<Id: Copy + Eq + Ord, Cid: Clone + Eq> PartialOrd for DocRef<Id, Cid> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Id: Copy + Eq + Ord, Cid: Clone + Eq> PartialEq for DocRef<Id, Cid> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(PartialEq, Eq, Encode, Decode, RuntimeDebug)]
/// Stores RFC governance rules and
pub struct RfcBoard<Id, OrgId, AccountId, ThresholdId, Doc> {
    id: Id,
    org: OrgId,
    controller: Option<AccountId>,
    threshold_id: ThresholdId,
    pub passed: OrderedSet<Doc>,
}

impl<
        CommitteeId: Copy,
        OrgId: Copy,
        AccountId: Clone + PartialEq,
        ThresholdId: Copy,
        Cid: Clone + Eq,
        Id: Copy + Zero + From<u32> + Ord + Eq,
    > RfcBoard<CommitteeId, OrgId, AccountId, ThresholdId, DocRef<Id, Cid>>
{
    pub fn new(
        id: CommitteeId,
        org: OrgId,
        controller: Option<AccountId>,
        threshold_id: ThresholdId,
    ) -> Self {
        Self {
            id,
            org,
            controller,
            threshold_id,
            passed: OrderedSet::new(),
        }
    }
    pub fn id(&self) -> CommitteeId {
        self.id
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn controller(&self) -> Option<AccountId> {
        self.controller.clone()
    }
    pub fn is_controller(&self, who: &AccountId) -> bool {
        if let Some(c) = self.controller.clone() {
            return who == &c
        }
        false
    }
    pub fn threshold_id(&self) -> ThresholdId {
        self.threshold_id
    }
    pub fn add_doc(mut self, c: Cid) -> Option<(Id, Self)> {
        if let Some(last_added_id) = self.passed.0.clone().iter().rev().next() {
            let id = last_added_id.id + 1u32.into();
            if self.passed.insert(DocRef::new(id, c)) {
                Some((id, self))
            } else {
                None
            }
        } else {
            let id = Id::zero() + 1u32.into();
            if self.passed.insert(DocRef::new(id, c)) {
                Some((id, self))
            } else {
                None
            }
        }
    }
}
