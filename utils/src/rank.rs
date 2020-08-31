use orml_utilities::OrderedSet;
use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum BallotState {
    NoVote,
    Voted,
}

#[derive(PartialEq, Eq, Encode, Decode, RuntimeDebug)]
/// Stores choices ordered by user
/// -> Choice: (AccountId, Signal)
pub struct Ballot<Key, AccountId, Signal, State> {
    key: Key,
    total: Signal,
    pub vote: OrderedSet<(AccountId, Signal)>,
    state: State,
}

impl<Key: Clone, AccountId: Clone + Ord, Signal: Copy + Ord>
    Ballot<Key, AccountId, Signal, BallotState>
{
    pub fn new(key: Key, total: Signal) -> Self {
        Self {
            key,
            total,
            vote: OrderedSet::new(),
            state: BallotState::NoVote,
        }
    }
    pub fn key(&self) -> Key {
        self.key.clone()
    }
    pub fn total(&self) -> Signal {
        self.total
    }
    pub fn state(&self) -> BallotState {
        self.state
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub enum VoteState {
    Open,
    Locked,
}

#[derive(PartialEq, Eq, Encode, Decode, RuntimeDebug)]
/// Stores choice options
pub struct VoteBoard<VoteId, Cid, AccountId, Signal, State> {
    id: VoteId,
    topic: Cid,
    controller: AccountId,
    pub choices: OrderedSet<(AccountId, Signal)>,
    state: State,
}

impl<VoteId: Copy, Cid: Clone, AccountId: Clone + PartialEq, Signal: Copy>
    VoteBoard<VoteId, Cid, AccountId, Signal, VoteState>
{
    pub fn new(
        id: VoteId,
        topic: Cid,
        controller: AccountId,
        choices: OrderedSet<(AccountId, Signal)>,
    ) -> Self {
        Self {
            id,
            topic,
            controller,
            choices,
            state: VoteState::Open,
        }
    }
    pub fn id(&self) -> VoteId {
        self.id
    }
    pub fn topic(&self) -> Cid {
        self.topic.clone()
    }
    pub fn controller(&self) -> AccountId {
        self.controller.clone()
    }
    pub fn is_controller(&self, who: &AccountId) -> bool {
        &self.controller == who
    }
    pub fn state(&self) -> VoteState {
        self.state
    }
    pub fn lock(self) -> Self {
        Self {
            state: VoteState::Locked,
            ..self
        }
    }
}
