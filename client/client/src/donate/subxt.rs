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
};

/// The donation balance type
pub type DonateBalanceOf<T> = <T as Donate>::DCurrency; // as Currency<<T as System>::AccountId>>::Balance;

#[module]
pub trait Donate: System + Org {
    /// The currency type for on-chain transactions
    type DCurrency: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero; // + Currency<<Self as System>::AccountId> // commented out until #93 is resolved
}

// ~~ Values (Constants) ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct TransactionFee<T: Donate> {
    pub amount: DonateBalanceOf<T>,
}

// ~~ Calls and Events ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MakePropDonationCall<T: Donate> {
    pub org: <T as Org>::OrgId,
    pub rem_recipient: <T as System>::AccountId,
    pub amt: DonateBalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct PropDonationExecutedEvent<T: Donate> {
    pub sender: <T as System>::AccountId,
    pub org: <T as Org>::OrgId,
    pub amt_to_org: DonateBalanceOf<T>,
    pub rem_recipient: <T as System>::AccountId,
    pub amt_to_recipient: DonateBalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MakeEqualDonationCall<T: Donate> {
    pub org: <T as Org>::OrgId,
    pub rem_recipient: <T as System>::AccountId,
    pub amt: DonateBalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct EqualDonationExecutedEvent<T: Donate> {
    pub sender: <T as System>::AccountId,
    pub org: <T as Org>::OrgId,
    pub amt_to_org: DonateBalanceOf<T>,
    pub rem_recipient: <T as System>::AccountId,
    pub amt_to_recipient: DonateBalanceOf<T>,
}
