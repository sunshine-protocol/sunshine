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
    ModuleId,
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
        bank<T>,
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
impl org::Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32; // TODO: replace with utils_identity::Cid
    type OrgId = u64;
    type Shares = u64;
    type ReservationLimit = ReservationLimit;
}
impl vote::Trait for Test {
    type Event = TestEvent;
    type VoteId = u64;
    type Signal = u64;
}
parameter_types! {
    pub const TransactionFee: u64 = 3;
    pub const TreasuryModuleId: ModuleId = ModuleId(*b"py/trsry");
}
impl donate::Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
    type TransactionFee = TransactionFee;
    type Treasury = TreasuryModuleId;
}
parameter_types! {
    pub const MaxTreasuryPerOrg: u32 = 50;
    pub const MinimumInitialDeposit: u64 = 20;
}
impl bank::Trait for Test {
    type Event = TestEvent;
    type SpendId = u64;
    type Currency = Balances;
    type MaxTreasuryPerOrg = MaxTreasuryPerOrg;
    type MinimumInitialDeposit = MinimumInitialDeposit;
}
parameter_types! {
    // minimum deposit to register an on-chain bank
    pub const BountyLowerBound: u64 = 5;
}
impl Trait for Test {
    type Event = TestEvent;
    type BountyId = u64;
    type BountyLowerBound = BountyLowerBound;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Bounty = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64, u64> {
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
fn account_posts_bounty_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_noop!(
            Bounty::account_posts_bounty(
                one.clone(),
                10u32, // constitution
                101,   // amount reserved for bounty
                new_resolution_metadata.clone(),
                None,
            ),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance",),
            }
        );
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_eq!(get_last_event(), RawEvent::BountyPosted(1, 1, 10));
    });
}

#[test]
fn account_applies_for_bounty_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_noop!(
            Bounty::account_applies_for_bounty(
                two.clone(),
                1,
                15u32, // application description
                11
            ),
            Error::<Test>::GrantApplicationRequestExceedsBountyFundingReserved
        );
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::BountyApplicationSubmitted(1, 1, 2, None, 10)
        );
    });
}

#[test]
fn account_triggers_application_review_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_noop!(
            Bounty::account_applies_for_bounty(
                two.clone(),
                1,
                15u32, // application description
                11
            ),
            Error::<Test>::GrantApplicationRequestExceedsBountyFundingReserved
        );
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_triggers_application_review(
            one.clone(),
            1,
            1,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationReviewTriggered(
                1,
                1,
                1,
                ApplicationState::UnderReviewByAcceptanceCommittee(1)
            )
        );
    });
}

#[test]
fn account_sudo_approves_application_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_noop!(
            Bounty::account_applies_for_bounty(
                two.clone(),
                1,
                15u32, // application description
                11
            ),
            Error::<Test>::GrantApplicationRequestExceedsBountyFundingReserved
        );
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_sudo_approves_application(
            one.clone(),
            1,
            1,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::SudoApprovedBountyApplication(
                1,
                1,
                1,
                ApplicationState::ApprovedAndLive
            )
        );
    });
}

#[test]
fn account_poll_application_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1,
                1,
                1,
                ApplicationState::SubmittedAwaitingResponse
            )
        );
        assert_ok!(Bounty::account_sudo_approves_application(
            one.clone(),
            1,
            1,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::SudoApprovedBountyApplication(
                1,
                1,
                1,
                ApplicationState::ApprovedAndLive
            )
        );
        assert_ok!(Bounty::account_poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1,
                1,
                1,
                ApplicationState::ApprovedAndLive
            )
        );
    });
}

#[test]
fn milestone_submission_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_sudo_approves_application(
            one.clone(),
            1,
            1,
        ));
        assert_noop!(
            Bounty::grantee_submits_milestone(
                two.clone(),
                2,
                1,
                10u32, // milestone reference
                10
            ),
            Error::<Test>::CannotSubmitMilestoneIfBaseBountyDNE
        );
        assert_noop!(
            Bounty::grantee_submits_milestone(
                two.clone(),
                1,
                2,
                10u32, // milestone reference
                10
            ),
            Error::<Test>::CannotSubmitMilestoneIfApplicationDNE
        );
        assert_ok!(Bounty::grantee_submits_milestone(
            two.clone(),
            1,
            1,
            10u32, // milestone reference
            10
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneSubmitted(2, 1, 1, 1, 10,)
        );
    });
}

#[test]
fn account_triggers_milestone_review_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_sudo_approves_application(
            one.clone(),
            1,
            1,
        ));
        assert_ok!(Bounty::grantee_submits_milestone(
            two.clone(),
            1,
            1,
            10u32, // milestone reference
            10
        ));
        assert_ok!(Bounty::account_triggers_milestone_review(
            one.clone(),
            1,
            1,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneReviewTriggered(
                1,
                1,
                1,
                MilestoneStatus::SubmittedReviewStarted(1)
            )
        );
    });
}

#[test]
fn account_sudo_approves_milestone_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let two = Origin::signed(2);
        let new_resolution_metadata = ResolutionMetadata::new(1, 1, None, None);
        assert_ok!(Bounty::account_posts_bounty(
            one.clone(),
            10u32, // constitution
            10,    // amount reserved for bounty
            new_resolution_metadata,
            None,
        ));
        assert_ok!(Bounty::account_applies_for_bounty(
            two.clone(),
            1,
            15u32, // application description
            10
        ));
        assert_ok!(Bounty::account_sudo_approves_application(
            one.clone(),
            1,
            1,
        ));
        assert_ok!(Bounty::grantee_submits_milestone(
            two.clone(),
            1,
            1,
            10u32, // milestone reference
            10
        ));
        assert_ok!(Bounty::account_approved_milestone(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::SudoApprovedMilestone(
                1,
                1,
                1,
                MilestoneStatus::ApprovedAndTransferExecuted,
            )
        );
    });
}
