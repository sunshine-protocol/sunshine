use crate::{
    donate::{
        Donate,
        DonateEventsDecoder,
    },
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
    Call,
    Event,
    Store,
};
use sunshine_bounty_utils::bank::{
    BankState,
    SpendState,
};

pub type BalanceOf<T> = <T as Balances>::Balance;

/// The subset of the bank trait and its inherited traits that the client must inherit
#[module]
pub trait Bank: System + Balances + Org + Vote + Donate {
    type BankId: Parameter
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
    type SpendId: Parameter
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
}

// ~~ Values (Constants) ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct MinimumInitialDepositStore<T: Bank> {
    pub amount: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct MinimumTransferStore<T: Bank> {
    pub amount: BalanceOf<T>,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct BankStoresStore<T: Bank> {
    #[store(returns = BankState<<T as System>::AccountId, <T as Org>::OrgId>)]
    pub id: T::BankId,
    phantom: std::marker::PhantomData<T>,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct OpenOrgBankAccountCall<T: Bank> {
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as System>::AccountId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct OrgBankAccountOpenedEvent<T: Bank> {
    pub seeder: <T as System>::AccountId,
    pub new_bank_id: T::BankId,
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as System>::AccountId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MemberProposesSpendCall<T: Bank> {
    pub bank_id: T::BankId,
    pub amount: BalanceOf<T>,
    pub dest: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SpendProposedByMemberEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub amount: BalanceOf<T>,
    pub dest: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MemberTriggersVoteOnSpendProposalCall<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct VoteTriggeredOnSpendProposalEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub vote_id: <T as Vote>::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MemberSudoApprovesSpendProposalCall<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedSpendProposalEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct MemberPollsSpendProposalCall<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SpendProposalPolledEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub state: SpendState<<T as Vote>::VoteId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CloseBankAccountCall<T: Bank> {
    pub bank_id: T::BankId,
    p: core::marker::PhantomData<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct BankAccountClosedEvent<T: Bank> {
    pub closer: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub org: <T as Org>::OrgId,
}
