use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
pub struct Dispute<AccountId, Currency, BlockNumber, VoteMetadata, State> {
    locker: AccountId,
    locked_funds: Currency,
    dispute_raiser: AccountId,
    resolution_metadata: VoteMetadata,
    state: State,
    expiry: Option<BlockNumber>,
}

impl<
        AccountId: Clone + PartialEq,
        Currency: Clone,
        BlockNumber: Copy,
        VoteMetadata: Clone,
        State: Copy,
    > Dispute<AccountId, Currency, BlockNumber, VoteMetadata, State>
{
    pub fn locker(&self) -> AccountId {
        self.locker.clone()
    }
    pub fn locked_funds(&self) -> Currency {
        self.locked_funds.clone()
    }
    pub fn dispute_raiser(&self) -> AccountId {
        self.dispute_raiser.clone()
    }
    pub fn can_raise_dispute(&self, who: &AccountId) -> bool {
        &self.dispute_raiser() == who
    }
    pub fn resolution_metadata(&self) -> VoteMetadata {
        self.resolution_metadata.clone()
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn expiry(&self) -> Option<BlockNumber> {
        self.expiry
    }
    pub fn set_state(&self, state: State) -> Self {
        Self {
            state,
            ..self.clone()
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum DisputeState<VoteId> {
    DisputeNotRaised,
    /// Dispute raised and vote dispatched without outcome
    DisputeRaisedAndVoteDispatched(VoteId),
    /// Outcome and time last checked and outcome updated
    DisputeRaisedAndAccepted(VoteId),
    /// Outcome and time last checked and outcome updated
    DisputeRaisedAndRejected(VoteId),
}

impl<VoteId> Default for DisputeState<VoteId> {
    fn default() -> DisputeState<VoteId> {
        DisputeState::DisputeNotRaised
    }
}
