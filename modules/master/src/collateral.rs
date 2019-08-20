use runtime_primitives::traits::{SimpleArithmetic, MaybeSerializeDebug};
use parity_scale_codec::{Codec, Encode, Decode};

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Debug))]
pub enum BondType<C> {
    // vote bond (for registering members for voting)
    VoteRegister,
    // vote fee for bond-based voting
    VoteFee,
    // collateral for application to the DAO
    Application,
    // sponsoring member's collateral
    Sponsor,
}

// I want the return type to depend on the bond in question (not a uniform return type)
pub trait Collateral: {
    type BondType: BondType;

    fn calculate_bond(&self, ) -> result::Result<Self::BondType, &'static str>;
}

// WANT LIST
// I want an enum of all the bond types and a trait for 
// (1) bonding the amount until some specified period with specific reward/punishment scenarios
// (2) returning the amount bonded in the correct currency (I want the currency for bonding to be polymorphic)
//
// (3) add behavior for actually bonding and unbonding the collateral (keeping track of it with this separate type)
// (4) add some automated Drop type for default scenarios or something, stay frosty
// (5) how could we add reputation?
