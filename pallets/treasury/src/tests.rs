use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    weights::Weight,
};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::IdentityLookup,
    Perbill,
};

// type aliases
pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

mod treasury {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        treasury<T>,
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
    type MaximumExtrinsicWeight = MaximumBlockWeight;
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
    type BaseCallFilter = ();
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
parameter_types! {
    pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
}
impl Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
    type TreasuryAddress = TreasuryModuleId;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Treasury = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::treasury(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .last()
        .unwrap()
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![(1, 100), (2, 98), (3, 200), (4, 75), (5, 10), (6, 69)],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert!(System::events().is_empty());
    });
}

#[test]
fn check_donation_accuracy() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        assert_eq!(Balances::total_balance(&2), 98);
    });
}
