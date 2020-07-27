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

/// Auxiliary method for simulating block time passing
fn run_to_block(n: u64) {
    while System::block_number() < n {
        Treasury::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    GenesisConfig::<Test> {
        minting_interval: 10,
        mint_amount: 10,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn verify_issuance_rate() {
    new_test_ext().execute_with(|| {
        let treasury_account_id = Treasury::account_id();
        assert_eq!(0, Balances::total_balance(&treasury_account_id));
        run_to_block(11);
        assert_eq!(10, Balances::total_balance(&treasury_account_id));
        run_to_block(21);
        assert_eq!(20, Balances::total_balance(&treasury_account_id));
        run_to_block(31);
        assert_eq!(30, Balances::total_balance(&treasury_account_id));
        run_to_block(39);
        assert_eq!(30, Balances::total_balance(&treasury_account_id));
        run_to_block(40);
        assert_eq!(30, Balances::total_balance(&treasury_account_id));
        run_to_block(41);
        assert_eq!(40, Balances::total_balance(&treasury_account_id));
    });
}
