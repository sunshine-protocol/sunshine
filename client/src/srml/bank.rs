use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
use util::{organization::Organization, share::ShareProfile};
use crate::srml::org::Org;

/// The subset of the bank trait and its inherited traits that the client must inherit
#[module]
pub trait Bank: System + Org {
    /// Cid type
    type IpfsReference: Parameter + Member + Default;

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
}