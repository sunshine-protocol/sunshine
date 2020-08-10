use crate::{
    org::{
        Org,
        OrgEventsDecoder,
    },
    vote::{
        Vote,
        VoteEventsDecoder,
    },
};
use codec::{
    Codec,
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
    balances::{
        Balances,
        BalancesEventsDecoder,
    },
    module,
    sp_runtime,
    system::{
        System,
        SystemEventsDecoder,
    },
};

pub type BalanceOf<T> = <T as Balances>::Balance;

/// The subset of the org trait and its inherited traits that the client must inherit
#[module]
pub trait Court: System + Balances + Org + Vote {
    /// Dispute Type Identifier
    type DisputeId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;
}

// ~~ Values (Constants) ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct MinimumDisputeAmount<T: Court> {
    pub amount: BalanceOf<T>,
}
