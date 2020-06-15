use crate::srml::{
    bank::{BalanceOf, Bank, BankEventsDecoder},
    org::{Org, OrgEventsDecoder},
    vote::{Vote, VoteEventsDecoder},
};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    Permill,
};
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
use util::{
    bank::OnChainTreasuryID,
    bounty::{
        ApplicationState, BountyInformation, GrantApplication, MilestoneStatus,
        MilestoneSubmission, ReviewBoard, TeamID,
    },
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
