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
use libipld::{
    cbor::DagCborCodec,
    codec::{
        Decode as DagEncode,
        Encode as DagDecode,
    },
};
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    PerThing,
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
use sunshine_bounty_utils::{
    organization::OrgRep,
    vote::{
        Threshold,
        ThresholdConfig,
        Vote as VoteVector,
        VoteState,
        XorThreshold,
    },
};

pub type ThreshConfig<T> = ThresholdConfig<
    <T as Vote>::ThresholdId,
    OrgRep<<T as Org>::OrgId>,
    XorThreshold<<T as Vote>::Signal, <T as Vote>::Percent>,
>;

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

    /// The threshold identifier
    type ThresholdId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    /// The type for percentage vote thresholds
    type Percent: 'static + PerThing + Codec + Send + Sync;

    /// Vote topic associated type, text block
    type VoteTopic: 'static
        + Codec
        + Default
        + Clone
        + DagEncode<DagCborCodec>
        + DagDecode<DagCborCodec>
        + Send
        + Sync;

    /// Vote views
    type VoterView: 'static
        + Codec
        + Default
        + Debug
        + Eq
        + Copy
        + Clone
        + Send
        + Sync;

    /// Vote justification
    type VoteJustification: 'static
        + Codec
        + Default
        + Clone
        + DagEncode<DagCborCodec>
        + DagDecode<DagCborCodec>
        + Send
        + Sync;
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
pub struct VoteLoggerStore<T: Vote> {
    #[store(returns = VoteVector<T::Signal, <T as Org>::IpfsReference>)]
    pub vote: T::VoteId,
    pub who: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct VoteThresholdsStore<T: Vote> {
    #[store(returns = ThreshConfig<T>)]
    pub threshold: T::ThresholdId,
}

// ~~ Calls ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CreateSignalVoteCall<T: Vote> {
    pub topic: Option<<T as Org>::IpfsReference>,
    pub organization: OrgRep<T::OrgId>,
    pub threshold: Threshold<T::Signal>,
    pub duration: Option<<T as System>::BlockNumber>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CreatePercentVoteCall<T: Vote> {
    pub topic: Option<<T as Org>::IpfsReference>,
    pub organization: OrgRep<T::OrgId>,
    pub threshold: Threshold<T::Percent>,
    pub duration: Option<<T as System>::BlockNumber>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct SubmitVoteCall<T: Vote> {
    pub vote_id: T::VoteId,
    pub direction: <T as Vote>::VoterView,
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
    pub view: <T as Vote>::VoterView,
}
