use super::*;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

// type aliases
pub type AccountId = u64;
pub type Shares = u64;
pub type Signal = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
}
impl frame_system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = BlockNumber;
    type Call = ();
    type Hash = H256;
    type Hashing = ::sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = TestEvent;
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type AvailableBlockRatio = AvailableBlockRatio;
    type MaximumBlockLength = MaximumBlockLength;
    type Version = ();
    type ModuleToIndex = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
}
parameter_types! {
    pub const ReservationLimit: u32 = 10000;
}
impl membership::Trait for Test {
    type Event = TestEvent;
}
impl shares_membership::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
}
impl shares_atomic::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
    type Shares = Shares;
    type ReservationLimit = ReservationLimit;
}
impl Trait for Test {
    type Event = TestEvent;
    type Signal = Signal;
    type OrgData = membership::Module<Test>;
    type FlatShareData = shares_membership::Module<Test>;
    type WeightedShareData = shares_atomic::Module<Test>;
}

mod vote_yesno {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        membership<T>,
        shares_membership<T>,
        shares_atomic<T>,
        vote_yesno<T>,
    }
}
pub type VoteYesNo = Module<Test>;
