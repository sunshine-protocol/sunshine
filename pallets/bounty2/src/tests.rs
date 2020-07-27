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

mod bounty {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        org<T>,
        vote<T>,
        donate<T>,
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
impl org::Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32;
    type OrgId = u64;
    type Shares = u64;
}
impl vote::Trait for Test {
    type Event = TestEvent;
    type VoteId = u64;
    type Signal = u64;
}
impl donate::Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
}
parameter_types! {
    pub const BountyLowerBound: u64 = 5;
    pub const ChallengePeriod: BlockNumber = 100;
}
impl Trait for Test {
    type Event = TestEvent;
    type BountyId = u64;
    type SubmissionId = u64;
    type BountyLowerBound = BountyLowerBound;
    type ChallengePeriod = ChallengePeriod;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Vote = vote::Module<Test>;
pub type Bounty2 = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64, u32, u64, u64, u64, u64> {
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
    org::GenesisConfig::<Test> {
        first_organization_supervisor: 1,
        first_organization_value_constitution: 1738,
        first_organization_flat_membership: vec![1, 2, 3, 4, 5, 6],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

use util::organization::Organization;

#[test]
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Org::organization_counter(), 1);
        let constitution = 1738;
        let expected_organization =
            Organization::new(Some(1), None, constitution);
        let org_in_storage = Org::organization_states(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

#[test]
fn post_bounty_works() {
    new_test_ext().execute_with(|| {
        let fifty_one_pct_threshold = PercentageThreshold::new(
            sp_runtime::Permill::from_percent(51),
            None,
        );
        // 51% threshold with equal weight for every account
        let dispute_resolution = ResolutionMetadata::new(
            Some(1),
            OrgRep::Equal(1),
            fifty_one_pct_threshold,
        );
        assert_noop!(
            Bounty2::post_bounty(
                Origin::signed(1),
                10u32, // constitution
                101,   // funding reserved
                dispute_resolution.clone()
            ),
            sp_runtime::DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance",),
            },
        );
        let fake_dispute_resolution = ResolutionMetadata::new(
            Some(1),
            OrgRep::Equal(2),
            fifty_one_pct_threshold,
        );
        assert_noop!(
            Bounty2::post_bounty(
                Origin::signed(1),
                10u32, // constitution
                10,    // funding reserved
                fake_dispute_resolution
            ),
            Error::<Test>::DisputeResolvingOrgMustExistToPostBounty,
        );
        assert_ok!(Bounty2::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            10,    // funding reserved
            dispute_resolution
        ));
    });
}

#[test]
fn submission_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bounty2::submit_for_bounty(
                Origin::signed(1),
                1,
                None,
                10u32,
                15u64,
            ),
            Error::<Test>::BountyDNE
        );
        let fifty_one_pct_threshold = PercentageThreshold::new(
            sp_runtime::Permill::from_percent(51),
            None,
        );
        // 51% threshold with equal weight for every account
        let dispute_resolution = ResolutionMetadata::new(
            Some(1),
            OrgRep::Equal(1),
            fifty_one_pct_threshold,
        );
        assert_ok!(Bounty2::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            10,    // funding reserved
            dispute_resolution
        ));
        assert_noop!(
            Bounty2::submit_for_bounty(
                Origin::signed(1),
                1,
                None,
                10u32,
                15u64,
            ),
            Error::<Test>::SubmissionRequestExceedsBounty,
        );
        assert_ok!(Bounty2::submit_for_bounty(
            Origin::signed(1),
            1,
            None,
            10u32,
            10u64,
        ));
    });
}

#[test]
fn submission_approval_works() {
    new_test_ext().execute_with(|| {
        let fifty_one_pct_threshold = PercentageThreshold::new(
            sp_runtime::Permill::from_percent(51),
            None,
        );
        // 51% threshold with equal weight for every account
        let dispute_resolution = ResolutionMetadata::new(
            Some(1),
            OrgRep::Equal(1),
            fifty_one_pct_threshold,
        );
        assert_ok!(Bounty2::post_bounty(
            Origin::signed(1),
            10u32, // constitution
            10,    // funding reserved
            dispute_resolution
        ));
        assert_ok!(Bounty2::submit_for_bounty(
            Origin::signed(1),
            1,
            None,
            10u32,
            10u64,
        ));
        assert_noop!(
            Bounty2::approve_bounty_submission(Origin::signed(1), 2,),
            Error::<Test>::SubmissionDNE
        );
        assert_noop!(
            Bounty2::approve_bounty_submission(Origin::signed(2), 1,),
            Error::<Test>::NotAuthorizedToApproveBountySubmissions
        );
        assert_ok!(Bounty2::approve_bounty_submission(Origin::signed(1), 1,));
    });
}
