use crate::srml::{
    bank::{Bank, BankEventsDecoder},
    org::{Org, OrgEventsDecoder},
    vote::{Vote, VoteEventsDecoder},
};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
//use util::bounty::{};

#[module]
pub trait Bounty: System + Org + Vote {
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

// // The Bank module inherited as an associated type
// type Bank: IDIsAvailable<OnChainTreasuryID>
// + IDIsAvailable<(OnChainTreasuryID, BankMapID, BankAssociatedId<Self>)>
// + GenerateUniqueID<OnChainTreasuryID>
// + SeededGenerateUniqueID<BankAssociatedId<Self>, (OnChainTreasuryID, BankMapID)>
// + OnChainBank
// + RegisterAccount<Self::OrgId, Self::AccountId, BalanceOf<Self>>
// + CalculateOwnership<Self::OrgId, Self::AccountId, BalanceOf<Self>, Permill>
// + DepositsAndSpends<BalanceOf<Self>>
// + CheckBankBalances<BalanceOf<Self>>
// + DepositIntoBank<Self::OrgId, Self::AccountId, BalanceOf<Self>, Self::IpfsReference>
// + ReservationMachine<Self::OrgId, Self::AccountId, BalanceOf<Self>, Self::IpfsReference>
// + ExecuteSpends<Self::OrgId, Self::AccountId, BalanceOf<Self>, Self::IpfsReference>
// + CommitAndTransfer<Self::OrgId, Self::AccountId, BalanceOf<Self>, Self::IpfsReference>
// + TermSheetExit<Self::AccountId, BalanceOf<Self>>;
