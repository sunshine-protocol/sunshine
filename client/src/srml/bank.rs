use crate::srml::{
    org::{Org, OrgEventsDecoder},
    vote::{Vote, VoteEventsDecoder},
};
use codec::{Codec, Decode, Encode};
use frame_support::{
    traits::{Currency, Get},
    Parameter,
};
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
use util::bank::{BankState, OnChainTreasuryID};

pub type BalanceOf<T> = <<T as Bank>::Currency as Currency<<T as System>::AccountId>>::Balance;

/// The subset of the bank trait and its inherited traits that the client must inherit
#[module]
pub trait Bank: System + Org + Vote {
    /// Identifier for bank-related maps
    type BankAssociatedId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// The currency type for on-chain transactions
    type Currency: Currency<<Self as System>::AccountId> + Clone + Default + Codec + Send + 'static;
}

// ~~ Values (Constants) ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct MinimumInitialDepositStore<T: Bank> {
    pub amount: BalanceOf<T>,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct BankStoreStore<T: Bank> {
    #[store(returns = BankState<<T as Org>::OrgId, BalanceOf<T>>)]
    pub id: OnChainTreasuryID,
    phantom: std::marker::PhantomData<T>,
}
