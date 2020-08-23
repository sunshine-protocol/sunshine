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
use sunshine_bounty_utils::{
    bank::{
        BankState,
        SpendProposal,
        SpendState,
    },
    organization::OrgRep,
    vote::{
        ThresholdInput,
        XorThreshold,
    },
};

pub type BalanceOf<T> = <T as Balances>::Balance;
pub type BankSt<T> = BankState<
    <T as Bank>::BankId,
    <T as System>::AccountId,
    <T as Org>::OrgId,
    <T as Vote>::ThresholdId,
>;
pub type Threshold<T> = ThresholdInput<
    OrgRep<<T as Org>::OrgId>,
    XorThreshold<<T as Vote>::Signal, <T as Vote>::Percent>,
>;
pub type SpendProp<T> = SpendProposal<
    <T as Bank>::BankId,
    <T as Bank>::SpendId,
    BalanceOf<T>,
    <T as System>::AccountId,
    SpendState<<T as Vote>::VoteId>,
>;

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

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct BanksStore<T: Bank> {
    #[store(returns = BankSt<T>)]
    pub id: T::BankId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct SpendProposalsStore<T: Bank> {
    #[store(returns = SpendProp<T>)]
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct OpenCall<T: Bank> {
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as System>::AccountId>,
    pub threshold: Threshold<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct AccountOpenedEvent<T: Bank> {
    pub seeder: <T as System>::AccountId,
    pub new_bank_id: T::BankId,
    pub seed: BalanceOf<T>,
    pub hosting_org: <T as Org>::OrgId,
    pub bank_operator: Option<<T as System>::AccountId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct ProposeSpendCall<T: Bank> {
    pub bank_id: T::BankId,
    pub amount: BalanceOf<T>,
    pub dest: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SpendProposedEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub amount: BalanceOf<T>,
    pub dest: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct TriggerVoteCall<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct VoteTriggeredEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub vote_id: <T as Vote>::VoteId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct SudoApproveCall<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedEvent<T: Bank> {
    pub caller: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct ProposalPolledEvent<T: Bank> {
    pub bank_id: T::BankId,
    pub spend_id: T::SpendId,
    pub state: SpendState<<T as Vote>::VoteId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct CloseCall<T: Bank> {
    pub bank_id: T::BankId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct AccountClosedEvent<T: Bank> {
    pub closer: <T as System>::AccountId,
    pub bank_id: T::BankId,
    pub org: <T as Org>::OrgId,
}
