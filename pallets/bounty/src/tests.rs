use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    weights::Weight,
};
use frame_system::{self as system,};
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

mod bounty {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        bounty<T>,
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
    pub const Foundation: ModuleId = ModuleId(*b"fundacon");
    pub const MinDeposit: u64 = 10;
    pub const MinContribution: u64 = 5;
}
impl Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32;
    type Currency = Balances;
    type BountyId = u64;
    type SubmissionId = u64;
    type Foundation = Foundation;
    type MinDeposit = MinDeposit;
    type MinContribution = MinContribution;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Bounty = Module<Test>;

fn get_last_event() -> RawEvent<u64, u32, u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::bounty(inner) = e {
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
fn post_bounty_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bounty::post_bounty(
                Origin::signed(1),
                10u32, // cid
                9,     // amount
            ),
            Error::<Test>::BountyPostMustExceedMinDeposit,
        );
        assert_noop!(
            Bounty::post_bounty(
                Origin::signed(1),
                10u32, // cid
                101,   // amount
            ),
            sp_runtime::DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance",),
            },
        );
        assert_ok!(Bounty::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            10,    // funding reserved
        ));
        assert_eq!(RawEvent::BountyPosted(1, 10, 1, 10), get_last_event());
    });
}

#[test]
fn contribution_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Bounty::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            10,    // funding reserved
        ));
        assert_noop!(
            Bounty::contribute_to_bounty(Origin::signed(2), 2, 5),
            Error::<Test>::BountyDNE
        );
        assert_noop!(
            Bounty::contribute_to_bounty(Origin::signed(2), 1, 4),
            Error::<Test>::ContributionMustExceedModuleMin
        );
        assert_noop!(
            Bounty::contribute_to_bounty(Origin::signed(2), 1, 99),
            sp_runtime::DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance",),
            },
        );
        assert_ok!(Bounty::contribute_to_bounty(Origin::signed(2), 1, 5));
        assert_eq!(
            RawEvent::BountyRaiseContribution(2, 5, 1, 15, 10),
            get_last_event()
        );
    });
}

#[test]
fn submission_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bounty::submit_for_bounty(Origin::signed(2), 1, 10u32, 15u64,),
            Error::<Test>::BountyDNE
        );
        assert_ok!(Bounty::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            21,    // funding reserved
        ));
        assert_noop!(
            Bounty::submit_for_bounty(Origin::signed(1), 1, 10u32, 15u64,),
            Error::<Test>::DepositerCannotSubmitForBounty
        );
        assert_noop!(
            Bounty::submit_for_bounty(Origin::signed(2), 1, 10u32, 22u64,),
            Error::<Test>::BountySubmissionExceedsTotalAvailableFunding,
        );
        assert_ok!(Bounty::submit_for_bounty(
            Origin::signed(2),
            1,
            10u32,
            10u64,
        ));
        assert_eq!(
            RawEvent::BountySubmissionPosted(2, 1, 10, 1, 10, 10),
            get_last_event()
        );
    });
}

#[test]
fn submission_approval_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bounty::approve_bounty_submission(Origin::signed(1), 1),
            Error::<Test>::SubmissionDNE
        );
        assert_ok!(Bounty::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            21,    // funding reserved
        ));
        assert_noop!(
            Bounty::approve_bounty_submission(Origin::signed(1), 1),
            Error::<Test>::SubmissionDNE
        );
        assert_ok!(Bounty::submit_for_bounty(
            Origin::signed(2),
            1,
            10u32,
            10u64,
        ));
        assert_noop!(
            Bounty::approve_bounty_submission(Origin::signed(2), 1),
            Error::<Test>::NotAuthorizedToApproveBountySubmissions
        );
        assert_eq!(Balances::total_balance(&2), 98);
        assert_eq!(Balances::total_balance(&1), 79);
        assert_ok!(Bounty::approve_bounty_submission(Origin::signed(1), 1));
        assert_eq!(Balances::total_balance(&2), 108);
        assert_eq!(Balances::total_balance(&1), 79);
    });
}
