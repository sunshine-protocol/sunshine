use crate::org::{
    Org,
    OrgEventsDecoder,
};
use codec::{
    Codec,
    Decode,
    Encode,
};
use frame_support::Parameter;
use sp_runtime::traits::{
    AtLeast32Bit,
    MaybeSerializeDeserialize,
    Member,
    Zero,
};
use std::fmt::Debug;
use substrate_subxt::{
    module,
    sp_runtime,
    system::{
        System,
        SystemEventsDecoder,
    },
    Call,
    Event,
    Store,
};
use sunshine_bounty_utils::vote::{
    Vote as VoteVector,
    VoteState,
    VoterView,
};

/// The subset of the `vote::Trait` that a client must implement.
#[module]
pub trait Vote: System + Org {
    /// The identifier for each vote; ProposalId => Vec<VoteId> s.t. sum(VoteId::Outcomes) => ProposalId::Outcome
    type VoteId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    /// The native type for vote strength
    type Signal: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;
}

// ~~ Values ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct VoteIdCounterStore<T: Vote> {
    pub nonce: T::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct OpenVoteCounterStore {
    pub counter: u32,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct VoteStateStore<T: Vote> {
    #[store(returns = VoteState<T::Signal, <T as System>::BlockNumber, <T as Org>::IpfsReference>)]
    pub vote: T::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct TotalSignalIssuanceStore<T: Vote> {
    #[store(returns = T::Signal)]
    pub vote: T::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct VoteLoggerStore<T: Vote> {
    #[store(returns = VoteVector<T::Signal, <T as Org>::IpfsReference>)]
    pub vote: T::VoteId,
    pub who: <T as System>::AccountId,
}

// ~~ Calls ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CreateSignalThresholdVoteCall<T: Vote> {
    pub topic: Option<<T as Org>::IpfsReference>,
    pub organization: T::OrgId,
    pub support_requirement: T::Signal,
    pub turnout_requirement: Option<T::Signal>,
    pub duration: Option<<T as System>::BlockNumber>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CreatePercentThresholdVoteCall<T: Vote> {
    pub topic: Option<<T as Org>::IpfsReference>,
    pub organization: T::OrgId,
    pub support_requirement: sp_runtime::Permill,
    pub turnout_requirement: Option<sp_runtime::Permill>,
    pub duration: Option<<T as System>::BlockNumber>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CreateUnanimousConsentVoteCall<T: Vote> {
    pub topic: Option<<T as Org>::IpfsReference>,
    pub organization: T::OrgId,
    pub duration: Option<<T as System>::BlockNumber>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct SubmitVoteCall<T: Vote> {
    pub vote_id: T::VoteId,
    pub direction: VoterView,
    pub justification: Option<<T as Org>::IpfsReference>,
}

// ~~ Events ~~

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewVoteStartedEvent<T: Vote> {
    pub caller: <T as System>::AccountId,
    pub org: T::OrgId,
    pub new_vote_id: T::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct VotedEvent<T: Vote> {
    pub vote_id: T::VoteId,
    pub voter: <T as System>::AccountId,
    pub view: VoterView,
}
