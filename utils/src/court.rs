use orml_utilities::OrderedSet;
use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::{
    cmp::Ordering,
    prelude::*,
};

#[derive(new, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub struct Threshold<Rank, Id> {
    pub rank: Rank,
    pub id: Id,
}

impl<Rank: Copy + Eq + Ord, Id: Copy + Eq> Ord for Threshold<Rank, Id> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank.cmp(&other.rank)
    }
}

impl<Rank: Copy + Eq + Ord, Id: Copy + Eq> Eq for Threshold<Rank, Id> {}

impl<Rank: Copy + Eq + Ord, Id: Copy + Eq> PartialOrd for Threshold<Rank, Id> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Rank: Copy + Eq + Ord, Id: Copy + Eq> PartialEq for Threshold<Rank, Id> {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank
    }
}

#[derive(PartialEq, Eq, Encode, Decode, RuntimeDebug)]
/// Sequence of vote metadata for resolving disputes
pub struct Court<Id, AccountId, Balance, Threshold> {
    id: Id,
    controller: Option<AccountId>,
    // reservation required to trigger vote or appeal to next court
    bond: Balance,
    pub vote_seq: OrderedSet<Threshold>,
}

impl<Id: Copy, AccountId: Clone, Balance: Copy, Threshold: Copy + Ord>
    Court<Id, AccountId, Balance, Threshold>
{
    pub fn new(
        id: Id,
        controller: Option<AccountId>,
        bond: Balance,
        vote_seq: Vec<Threshold>,
    ) -> Self {
        Self {
            id,
            controller,
            bond,
            vote_seq: OrderedSet::from(vote_seq),
        }
    }
}
