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
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    Permill,
};
use std::fmt::Debug;
use substrate_subxt::system::{
    System,
    SystemEventsDecoder,
};
use util::{
    bank::OnChainTreasuryID,
    bounty::{
        ApplicationState,
        BountyInformation,
        GrantApplication,
        MilestoneStatus,
        MilestoneSubmission,
        ReviewBoard,
        TeamID,
    },
    organization::TermsOfAgreement,
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
pub struct MinimumBountyCollateralRatioConstant {
    pub get: Permill,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct BountyLowerBoundConstant<T: Bounty> {
    pub get: BalanceOf<T>,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct RegisteredFoundationsStore<T: Bounty> {
    #[store(returns = bool)]
    pub org: <T as Org>::OrgId,
    pub treasury_id: OnChainTreasuryID,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct FoundationSponsoredBountiesStore<T: Bounty> {
    #[store(returns = BountyInformation<<T as Org>::OrgId, <T as Bank>::BankAssociatedId, <T as Org>::IpfsReference, BalanceOf<T>, ReviewBoard<<T as Org>::OrgId, <T as System>::AccountId, <T as Org>::IpfsReference, ThresholdConfig<<T as Vote>::Signal, Permill>>>)]
    pub id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct BountyApplicationsStore<T: Bounty> {
    #[store(returns = GrantApplication<<T as System>::AccountId, <T as Org>::Shares, BalanceOf<T>, <T as Org>::IpfsReference, ApplicationState<TeamID<<T as Org>::OrgId, <T as System>::AccountId>, <T as Vote>::VoteId>>)]
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct MilestoneSubmissionsStore<T: Bounty> {
    #[store(returns = MilestoneSubmission<<T as Org>::IpfsReference, BalanceOf<T>, <T as System>::AccountId, T::BountyId, MilestoneStatus<<T as Org>::OrgId, <T as Vote>::VoteId, <T as Bank>::BankAssociatedId>>)]
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectRegisterFoundationFromExistingBankCall<T: Bounty> {
    pub registered_organization: <T as Org>::OrgId,
    pub bank_account: OnChainTreasuryID,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct FoundationRegisteredFromOnChainBankEvent<T: Bounty> {
    pub registered_organization: <T as Org>::OrgId,
    pub bank_account: OnChainTreasuryID,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectCreateBountyCall<T: Bounty> {
    pub registered_organization: <T as Org>::OrgId,
    pub description: <T as Org>::IpfsReference,
    pub bank_account: OnChainTreasuryID,
    pub amount_reserved_for_bounty: BalanceOf<T>,
    pub amount_claimed_available: BalanceOf<T>,
    pub acceptance_committee: ReviewBoard<
        <T as Org>::OrgId,
        <T as System>::AccountId,
        <T as Org>::IpfsReference,
        ThresholdConfig<<T as Vote>::Signal, Permill>,
    >,
    pub supervision_committee: Option<
        ReviewBoard<
            <T as Org>::OrgId,
            <T as System>::AccountId,
            <T as Org>::IpfsReference,
            ThresholdConfig<<T as Vote>::Signal, Permill>,
        >,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct FoundationPostedBountyEvent<T: Bounty> {
    pub bounty_creator: <T as System>::AccountId,
    pub registered_organization: <T as Org>::OrgId,
    pub bounty_identifier: T::BountyId,
    pub bank_account: OnChainTreasuryID,
    pub description: <T as Org>::IpfsReference,
    pub amount_reserved_for_bounty: BalanceOf<T>,
    pub amount_claimed_available: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectSubmitGrantApplicationCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub description: <T as Org>::IpfsReference,
    pub total_amount: BalanceOf<T>,
    pub terms_of_agreement: TermsOfAgreement<
        <T as System>::AccountId,
        <T as Org>::Shares,
        <T as Org>::IpfsReference,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct GrantApplicationSubmittedForBountyEvent<T: Bounty> {
    pub submitter: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub new_grant_app_id: T::BountyId,
    pub description: <T as Org>::IpfsReference,
    pub total_amount: BalanceOf<T>,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectTriggerApplicationReviewCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct ApplicationReviewTriggeredEvent<T: Bounty> {
    pub trigger: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<
        TeamID<<T as Org>::OrgId, <T as System>::AccountId>,
        <T as Vote>::VoteId,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectSudoApproveApplicationCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedApplicationEvent<T: Bounty> {
    pub purported_sudo: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<
        TeamID<<T as Org>::OrgId, <T as System>::AccountId>,
        <T as Vote>::VoteId,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct AnyAccPollApplicationCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct ApplicationPolledEvent<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub application_id: T::BountyId,
    pub application_state: ApplicationState<
        TeamID<<T as Org>::OrgId, <T as System>::AccountId>,
        <T as Vote>::VoteId,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectSubmitMilestoneCall<T: Bounty> {
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
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectTriggerMilestoneReviewCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MilestoneReviewTriggeredEvent<T: Bounty> {
    pub trigger: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        <T as Org>::OrgId,
        <T as Vote>::VoteId,
        <T as Bank>::BankAssociatedId,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectSudoApprovesMilestoneCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SudoApprovedMilestoneEvent<T: Bounty> {
    pub purported_sudo: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        <T as Org>::OrgId,
        <T as Vote>::VoteId,
        <T as Bank>::BankAssociatedId,
    >,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct DirectPollMilestoneCall<T: Bounty> {
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct MilestonePolledEvent<T: Bounty> {
    pub poller: <T as System>::AccountId,
    pub bounty_id: T::BountyId,
    pub milestone_id: T::BountyId,
    pub milestone_state: MilestoneStatus<
        <T as Org>::OrgId,
        <T as Vote>::VoteId,
        <T as Bank>::BankAssociatedId,
    >,
}
