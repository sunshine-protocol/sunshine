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

mod bank {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        membership<T>,
        shares_membership<T>,
        shares_atomic<T>,
        vote_yesno<T>,
        vote_petition<T>,
        bank<T>,
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const ReservationLimit: u32 = 10000;
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
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for Test {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}
impl membership::Trait for Test {
    type Event = TestEvent;
}
impl shares_membership::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
}
impl vote_petition::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
    type ShareData = shares_membership::Module<Test>;
}
impl shares_atomic::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
    type Shares = Shares;
    type ReservationLimit = ReservationLimit;
}
impl vote_yesno::Trait for Test {
    type Event = TestEvent;
    type Signal = Signal;
    type OrgData = membership::Module<Test>;
    type FlatShareData = shares_membership::Module<Test>;
    type WeightedShareData = shares_atomic::Module<Test>;
}
impl Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
    type OrgData = OrgMembership;
    type FlatShareData = FlatShareData;
    type VotePetition = VotePetition;
    type WeightedShareData = WeightedShareData;
    type VoteYesNo = VoteYesNo;
}
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type OrgMembership = membership::Module<Test>;
pub type FlatShareData = shares_membership::Module<Test>;
pub type WeightedShareData = shares_atomic::Module<Test>;
pub type VotePetition = vote_petition::Module<Test>;
pub type VoteYesNo = vote_yesno::Module<Test>;
pub type Bank = Module<Test>;
