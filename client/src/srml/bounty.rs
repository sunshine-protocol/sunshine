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
// use util::{
//     // bounty::{},
// };

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
