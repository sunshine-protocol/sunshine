use crate::{
    Bank,
    Bounty,
    Donate,
    Org,
    Vote,
};
use core::fmt::Debug;
use sp_runtime::{
    generic::Header,
    traits::{
        BlakeTwo256,
        IdentifyAccount,
        Verify,
    },
    MultiSignature,
    OpaqueExtrinsic,
};
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
    },
    system::System,
};
use utils_identity::cid::CidBytes;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Runtime;

impl System for Runtime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId =
        <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = pallet_indices::address::Address<Self::AccountId, u64>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for Runtime {
    type Balance = u128;
}

impl Org for Runtime {
    type IpfsReference = CidBytes;
    type OrgId = u64;
    type Shares = u64;
}

impl Vote for Runtime {
    type VoteId = u64;
    type Signal = u64;
}

impl Donate for Runtime {
    type DCurrency = u128;
}

impl Bank for Runtime {
    type SpendId = u64;
    type Currency = u128;
}

impl Bounty for Runtime {
    type BountyId = u64;
}

impl substrate_subxt::Runtime for Runtime {
    type Signature = MultiSignature;
    type Extra = substrate_subxt::DefaultExtra<Self>;
}
