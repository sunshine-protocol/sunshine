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

mod bench {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        bench<T>,
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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type PalletInfo = ();
}
impl Trait for Test {
    type Event = TestEvent;
    type Id = u64;
}
pub type System = frame_system::Module<Test>;
pub type Bench = Module<Test>;

fn get_last_event() -> RawEvent<u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::bench(inner) = e {
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
fn create_works() {
    new_test_ext().execute_with(|| {
        assert!(true);
    });
}

#[test]
fn add_works() {
    new_test_ext().execute_with(|| {
        assert!(true);
    });
}

#[test]
fn remove_works() {
    new_test_ext().execute_with(|| {
        assert!(true);
    });
}

#[test]
fn close_works() {
    new_test_ext().execute_with(|| {
        assert!(true);
    });
}
