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
use util::bank::{
    BankState, DepositInfo, InternalTransferInfo, OnChainTreasuryID, ReservationInfo,
};

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
pub struct BankStoresStore<T: Bank> {
    #[store(returns = BankState<<T as Org>::OrgId, BalanceOf<T>>)]
    pub id: OnChainTreasuryID,
    phantom: std::marker::PhantomData<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct DepositsStore<T: Bank> {
    #[store(returns = DepositInfo<<T as System>::AccountId, <T as Org>::IpfsReference, BalanceOf<T>>)]
    pub bank_id: OnChainTreasuryID,
    pub deposit_id: T::BankAssociatedId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct SpendReservationsStore<T: Bank> {
    #[store(returns = ReservationInfo<<T as Org>::IpfsReference, BalanceOf<T>, <T as Org>::OrgId>)]
    pub bank_id: OnChainTreasuryID,
    pub reservation_id: T::BankAssociatedId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct InternalTransfersStore<T: Bank> {
    #[store(returns = InternalTransferInfo<T::BankAssociatedId, <T as Org>::IpfsReference, BalanceOf<T>, <T as Org>::OrgId>)]
    pub bank_id: OnChainTreasuryID,
    pub transfer_id: T::BankAssociatedId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct RegisterAndSeedForBankAccountCall<T: Bank> {
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as Org>::OrgId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct RegisteredNewOnChainBankEvent<T: Bank> {
    pub seeder: <T as System>::AccountId,
    pub new_bank_id: OnChainTreasuryID,
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as Org>::OrgId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct ReserveSpendForBankAccountCall<T: Bank> {
    pub bank_id: OnChainTreasuryID,
    pub reason: <T as Org>::IpfsReference,
    pub controller: <T as Org>::OrgId,
}
