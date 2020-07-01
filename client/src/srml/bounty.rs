use crate::srml::{
    bank::{
        BalanceOf,
        Bank,
        BankEventsDecoder,
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
use substrate_subxt::system::{
    System,
    SystemEventsDecoder,
};
use util::{
    bank::{
        BankOrAccount,
        OnChainTreasuryID,
        TransferId,
    },
    bounty::{
        ApplicationState,
        BankSpend,
        BountyInformation,
        GrantApplication,
        MilestoneStatus,
        MilestoneSubmission,
    },
    court::ResolutionMetadata,
    vote::ThresholdConfig,
};

#[module]
pub trait Bounty: System + Org + Vote + Bank {
    /// Identifier for bounty-related maps and submaps
    type BountyId: Parameter
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

// ~~ Constants ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct BountyLowerBoundConstant<T: Bounty> {
    pub get: BalanceOf<T>,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct LiveBountiesStore<T: Bounty> {
    #[store(returns = BountyInformation<
        BankOrAccount<
            BankSpend<TransferId<T::BankId>>,
            T::AccountId
        >,
        T::IpfsReference,
        BalanceOf<T>,
        ResolutionMetadata<
            T::OrgId,
            ThresholdConfig<T::Signal>,
            T::BlockNumber,
        >,
    >)]
    pub id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct BountyApplicationsStore<T: Bounty> {
    #[store(returns = GrantApplication<
        T::AccountId,
        OnChainTreasuryID,
        BalanceOf<T>,
        T::IpfsReference,
        ApplicationState<T::VoteId>,
    >)]
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct MilestoneSubmissionsStore<T: Bounty> {
    #[store(returns = MilestoneSubmission<
        T::AccountId,
        T::BountyId,
        T::IpfsReference,
        BalanceOf<T>,
        MilestoneStatus<T::VoteId, BankOrAccount<TransferId<T::BankId>, T::AccountId>>
    >)]
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct AccountPostsBountyCall<T: Bounty> {
    pub description: <T as Org>::IpfsReference,
    pub amount_reserved_for_bounty: BalanceOf<T>,
    pub acceptance_committee: ResolutionMetadata<
        <T as Org>::OrgId,
        ThresholdConfig<<T as Vote>::Signal>,
        <T as System>::BlockNumber,
    >,
    pub supervision_committee: Option<
        ResolutionMetadata<
            <T as Org>::OrgId,
            ThresholdConfig<<T as Vote>::Signal>,
            <T as System>::BlockNumber,
        >,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct BountyPostedEvent<T: Bounty> {
    pub new_bounty_id: T::BountyId,
    pub poster: <T as System>::AccountId,
    pub amount_reserved_for_bounty: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct AccountAppliesForBountyCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub description: <T as Org>::IpfsReference,
    pub total_amount: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct BountyApplicationSubmittedEvent<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub new_grant_app_id: T::BountyId,
    pub submitter: <T as System>::AccountId,
    pub org_bank: Option<OnChainTreasuryID>,
    pub total_amount: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct AccountTriggersApplicationReviewCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub new_grant_app_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct ApplicationReviewTriggeredEvent<T: Bounty> {
    pub trigger: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<<T as Vote>::VoteId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct AccountSudoApprovedApplicationCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedApplicationEvent<T: Bounty> {
    pub sudo: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<<T as Vote>::VoteId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct PollApplicationCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct PollApplicationEvent<T: Bounty> {
    pub poller: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<<T as Vote>::VoteId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct SubmitMilestoneCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub submission_reference: <T as Org>::IpfsReference,
    pub amount_requested: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MilestoneSubmittedEvent<T: Bounty> {
    pub submitter: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub new_milestone_id: T::BountyId,
    pub amount_requested: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct TriggerMilestoneReviewCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MilestoneReviewTriggeredEvent<T: Bounty> {
    pub trigger: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        T::VoteId,
        BankOrAccount<TransferId<T::BankId>, T::AccountId>,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct SudoApprovesMilestoneCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedMilestoneEvent<T: Bounty> {
    pub sudo: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        T::VoteId,
        BankOrAccount<TransferId<T::BankId>, T::AccountId>,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct PollMilestoneCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MilestonePolledEvent<T: Bounty> {
    pub poller: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        T::VoteId,
        BankOrAccount<TransferId<T::BankId>, T::AccountId>,
    >,
}
