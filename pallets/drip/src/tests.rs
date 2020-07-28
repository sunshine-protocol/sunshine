use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    traits::OnFinalize,
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

mod drip {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        drip<T>,
    }
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
    type SystemWeightInfo = ();
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
    type WeightInfo = ();
}
impl Trait for Test {
    type Event = TestEvent;
    type DripId = u64;
    type Currency = Balances;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Drip = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::drip(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .last()
        .unwrap()
}

/// Auxiliary method for simulating block time passing
fn run_to_block(n: u64) {
    while System::block_number() < n {
        Drip::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    pallet_balances::GenesisConfig::<Test> {
        balances: vec![
            (1, 1000),
            (2, 100),
            (3, 100),
            (4, 100),
            (5, 100),
            (6, 100),
        ],
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
fn drip_started() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::total_balance(&1), 1000);
        assert_eq!(Balances::total_balance(&2), 100);
        let ten_ten = DripRate::new(10, 10);
        let zero_ten = DripRate::new(10, 0);
        assert_noop!(
            Drip::start_drip(Origin::signed(1), 2, zero_ten),
            Error::<Test>::RatePeriodLengthMustBeGreaterThanZero
        );
        assert_noop!(
            Drip::start_drip(Origin::signed(1), 1, ten_ten),
            Error::<Test>::DoNotDripToSelf
        );
        assert_noop!(
            Drip::start_drip(Origin::signed(1), 1, ten_ten),
            Error::<Test>::DoNotDripToSelf
        );
        System::set_block_number(8);
        assert_ok!(Drip::start_drip(Origin::signed(1), 2, ten_ten));
        run_to_block(14);
        assert_eq!(Balances::total_balance(&1), 990);
        assert_eq!(Balances::total_balance(&2), 110);
        run_to_block(21);
        assert_eq!(Balances::total_balance(&1), 980);
        assert_eq!(Balances::total_balance(&2), 120);
        run_to_block(31);
        assert_eq!(Balances::total_balance(&1), 970);
        assert_eq!(Balances::total_balance(&2), 130);
    });
}

#[test]
fn drip_cancelled() {
    new_test_ext().execute_with(|| {
        assert_eq!(Balances::total_balance(&1), 1000);
        assert_eq!(Balances::total_balance(&2), 100);
        let ten_ten = DripRate::new(10, 10);
        System::set_block_number(8);
        assert_ok!(Drip::start_drip(Origin::signed(1), 2, ten_ten));
        run_to_block(14);
        assert_eq!(Balances::total_balance(&1), 990);
        assert_eq!(Balances::total_balance(&2), 110);
        assert_noop!(
            Drip::cancel_drip(Origin::signed(2), 1),
            Error::<Test>::NotAuthorizedToCancelDrip
        );
        run_to_block(21);
        assert_eq!(Balances::total_balance(&1), 980);
        assert_eq!(Balances::total_balance(&2), 120);
        assert_noop!(
            Drip::cancel_drip(Origin::signed(1), 2),
            Error::<Test>::DripDNE
        );
        assert_ok!(Drip::cancel_drip(Origin::signed(1), 1));
        run_to_block(31);
        assert_eq!(Balances::total_balance(&1), 980);
        assert_eq!(Balances::total_balance(&2), 120);
    });
}
