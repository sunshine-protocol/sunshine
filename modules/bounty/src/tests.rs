#![cfg(test)]

use super::*;
use frame_support::{assert_err, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

// type aliases
pub type AccountId = u64;
type OrgId = u32;
type FlatShareId = u32;
type WeightedShareId = u32;
pub type Shares = u64;
pub type Signal = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

mod bounty {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        system<T>,
        pallet_balances<T>,
        membership<T>,
        shares_atomic<T>,
        shares_membership<T>,
        org<T>,
        bank<T>,
        vote_yesno<T>,
        vote_petition<T>,
        bounty<T>,
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct TestRuntime;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const ReservationLimit: u32 = 10000;
}
impl frame_system::Trait for TestRuntime {
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
impl pallet_balances::Trait for TestRuntime {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}
impl membership::Trait for TestRuntime {
    type Event = TestEvent;
}
impl shares_membership::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
}
impl shares_atomic::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
    type Shares = Shares;
    type ReservationLimit = ReservationLimit;
}
impl org::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgId = OrgId;
    type FlatShareId = FlatShareId;
    type WeightedShareId = WeightedShareId;
    type OrgData = OrgMembership;
    type FlatShareData = FlatShareData;
    type WeightedShareData = WeightedShareData;
}
impl bank::Trait for TestRuntime {
    type Event = TestEvent;
    type Currency = Balances;
    type Organization = OrganizationInterface;
}
impl vote_petition::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
    type ShareData = shares_membership::Module<TestRuntime>;
}
impl vote_yesno::Trait for TestRuntime {
    type Event = TestEvent;
    type Signal = Signal;
    type OrgData = membership::Module<TestRuntime>;
    type FlatShareData = shares_membership::Module<TestRuntime>;
    type WeightedShareData = shares_atomic::Module<TestRuntime>;
}
parameter_types! {
    pub const MinimumBountyCollateralRatio: Permill = Permill::from_percent(20);
    pub const BountyLowerBound: u64 = 100;
}
impl Trait for TestRuntime {
    type Event = TestEvent;
    type Currency = Balances;
    type Organization = OrganizationInterface;
    type VotePetition = VotePetition;
    type VoteYesNo = VoteYesNo;
    type Bank = Bank;
    type MinimumBountyCollateralRatio = MinimumBountyCollateralRatio;
    type BountyLowerBound = BountyLowerBound;
}
pub type System = system::Module<TestRuntime>;
pub type Balances = pallet_balances::Module<TestRuntime>;
pub type OrgMembership = membership::Module<TestRuntime>;
pub type FlatShareData = shares_membership::Module<TestRuntime>;
pub type WeightedShareData = shares_atomic::Module<TestRuntime>;
pub type OrganizationInterface = org::Module<TestRuntime>;
pub type Bank = bank::Module<TestRuntime>;
pub type VoteYesNo = vote_yesno::Module<TestRuntime>;
pub type VotePetition = vote_petition::Module<TestRuntime>;
pub type Bounty = Module<TestRuntime>;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn test_event_emittance() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bounty::fake_method(one));
        // // this should actually work
        let expected_event = TestEvent::bounty(RawEvent::PlaceHolder(1));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        // assert_eq!(
        //     System::events().into_iter().map(|r| r.event)
        // 		.filter_map(|e| {
        // 			if let TestEvent::bounty(inner) = e { Some(inner) } else { None }
        // 		})
        // 		.last()
        // 		.unwrap(),
        // 	RawEvent::PlaceHolder(1),
        // );
    });
}
