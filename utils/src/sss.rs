use codec::{
    Decode,
    Encode,
};
use orml_utilities::OrderedSet;
use sp_runtime::RuntimeDebug;
use sp_std::{
    cmp::{
        Eq,
        Ordering,
    },
    prelude::*,
};

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub enum RelationState {
    Unreserved,
    ReservedCollateral,
}

impl Default for RelationState {
    fn default() -> RelationState {
        RelationState::Unreserved
    }
}

#[derive(new, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct Relation<Key, Round, State> {
    pub key: Key,
    pub history: OrderedSet<Round>,
    pub state: State,
}

#[derive(Clone, Encode, Decode, RuntimeDebug)]
pub struct Commit<RoundId, Hash, PreImage> {
    round_id: RoundId,
    hash: Hash,
    reveal: Option<PreImage>,
}

impl<RoundId: Copy + Eq + Ord, Hash: Clone + Eq, PreImage: Clone + Eq> Eq
    for Commit<RoundId, Hash, PreImage>
{
}

impl<RoundId: Copy + Eq + Ord, Hash: Clone + Eq, PreImage: Clone + Eq>
    Commit<RoundId, Hash, PreImage>
{
    pub fn round_id(&self) -> RoundId {
        self.round_id
    }
    pub fn hash(&self) -> Hash {
        self.hash.clone()
    }
    pub fn reveal(&self) -> Option<PreImage> {
        self.reveal.clone()
    }
}

impl<RoundId: Copy + Eq + Ord, Hash: Clone + Eq, PreImage: Clone + Eq> Ord
    for Commit<RoundId, Hash, PreImage>
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.round_id.cmp(&other.round_id)
    }
}

impl<RoundId: Copy + Eq + Ord, Hash: Clone + Eq, PreImage: Clone + Eq>
    PartialOrd for Commit<RoundId, Hash, PreImage>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<RoundId: Copy + Eq + Ord, Hash: Clone + Eq, PreImage: Clone + Eq> PartialEq
    for Commit<RoundId, Hash, PreImage>
{
    fn eq(&self, other: &Self) -> bool {
        self.round_id == other.round_id
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
/// Tracks whether the user has invoked the secret sharing network for the round's recovery
pub enum SSSState {
    Unused,
    UsedWithSuccess,
    UsedWithoutSuccess,
}

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct SecretState<Id, AccountId, RoundId, Balance, State> {
    id: Id,
    user: AccountId,
    round: RoundId,
    reserve_req: Balance,
    state: State,
}

impl<
        Id: Copy,
        AccountId: Clone,
        RoundId: Copy + sp_std::ops::Add<Output = RoundId> + From<u32>,
        Balance: Copy,
        State: Copy,
    > SecretState<Id, AccountId, RoundId, Balance, State>
{
    pub fn id(&self) -> Id {
        self.id
    }
    pub fn user(&self) -> AccountId {
        self.user.clone()
    }
    pub fn round(&self) -> RoundId {
        self.round
    }
    pub fn inc_round(&self) -> Self {
        Self {
            round: self.round + 1u32.into(),
            ..self.clone()
        }
    }
    pub fn reserve_req(&self) -> Balance {
        self.reserve_req
    }
    pub fn state(&self) -> State {
        self.state
    }
}
