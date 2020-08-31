use crate::org::{
    Org,
    OrgEventsDecoder,
};
use parity_scale_codec::{
    Decode,
    Encode,
};
use std::fmt::Debug;
use substrate_subxt::{
    balances::{
        Balances,
        BalancesEventsDecoder,
    },
    module,
    system::{
        System,
        SystemEventsDecoder,
    },
    Call,
    Event,
};

/// The balance type
pub type BalanceOf<T> = <T as Balances>::Balance;

#[module]
pub trait Donate: System + Balances + Org {}

// ~~ Values (Constants) ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct TransactionFee<T: Donate> {
    pub amount: BalanceOf<T>,
}

// ~~ Calls and Events ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MakePropDonationCall<T: Donate> {
    pub org: <T as Org>::OrgId,
    pub rem_recipient: <T as System>::AccountId,
    pub amt: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct PropDonationExecutedEvent<T: Donate> {
    pub sender: <T as System>::AccountId,
    pub org: <T as Org>::OrgId,
    pub amt_to_org: BalanceOf<T>,
    pub rem_recipient: <T as System>::AccountId,
    pub amt_to_recipient: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MakeEqualDonationCall<T: Donate> {
    pub org: <T as Org>::OrgId,
    pub rem_recipient: <T as System>::AccountId,
    pub amt: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct EqualDonationExecutedEvent<T: Donate> {
    pub sender: <T as System>::AccountId,
    pub org: <T as Org>::OrgId,
    pub amt_to_org: BalanceOf<T>,
    pub rem_recipient: <T as System>::AccountId,
    pub amt_to_recipient: BalanceOf<T>,
}
