use crate::srml::org::{
    Org,
    OrgEventsDecoder,
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
use substrate_subxt::system::{
    System,
    SystemEventsDecoder,
};
use util::bank::OnChainTreasuryID;

pub type BalanceOf<T> = <T as Donate>::Currency; // as Currency<<T as System>::AccountId>>::Balance;

#[module]
pub trait Donate: System + Org {
    /// The currency type for on-chain transactions
    type Currency: Parameter
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
    pub amount: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct TreasuryAddress {
    // ModuleId type which implements Debug
    pub module_id: OnChainTreasuryID,
}
