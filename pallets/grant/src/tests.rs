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
use util::{
    grant::Recipient,
    meta::{
        ResolutionMetadata,
        VoteCall,
        VoteMetadata,
    },
    organization::{
        OrgRep,
        Organization,
    },
    traits::GroupMembership,
    vote::Threshold,
};

// type aliases
pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

mod grant {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        pallet_balances<T>,
        org<T>,
        vote<T>,
        donate<T>,
        grant<T>,
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
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
}
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
    pub const MaxLocks: u32 = 50;
}
impl pallet_balances::Trait for Test {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = MaxLocks;
    type AccountStore = System;
    type WeightInfo = ();
}
impl org::Trait for Test {
    type Event = TestEvent;
    type Cid = u32;
    type OrgId = u64;
    type Shares = u64;
}
impl vote::Trait for Test {
    type Event = TestEvent;
    type VoteId = u64;
    type Signal = u64;
    type ThresholdId = u64;
}
impl donate::Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
}
parameter_types! {
    pub const BigFoundation: ModuleId = ModuleId(*b"big/fund");
    pub const MinDeposit: u64 = 20;
    pub const MinContribution: u64 = 10;
}
impl Trait for Test {
    type Event = TestEvent;
    type FoundationId = u64;
    type ApplicationId = u64;
    type MilestoneId = u64;
    type BigFoundation = BigFoundation;
    type MinDeposit = MinDeposit;
    type MinContribution = MinContribution;
}
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Grant = Module<Test>;

fn get_last_event(
) -> RawEvent<u64, u32, u64, u64, u64, u64, u64, Recipient<u64, OrgRep<u64>>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::grant(inner) = e {
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
        sudo: 1,
        doc: 1738,
        mems: vec![1, 2, 3, 4, 5, 6],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    GenesisConfig::<Test> {
        application_poll_frequency: 10,
        milestone_poll_frequency: 10,
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
        assert_eq!(Org::org_counter(), 1);
        let constitution = 1738;
        let expected_organization =
            Organization::new(Some(1), 1, 6, constitution);
        let org_in_storage = Org::orgs(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

fn sudo_threshold_no_vote() -> GovernanceOf<Test> {
    ResolutionMetadata::new(Some(1u64), None).unwrap()
}

fn new_min_threshold_and_sudo() -> GovernanceOf<Test> {
    ResolutionMetadata::new(
        Some(1u64),
        Some(VoteMetadata::Signal(VoteCall::new(
            OrgRep::Equal(1u64),
            Threshold::new(1u64, None),
            None,
        ))),
    )
    .unwrap()
}

fn new_min_threshold_no_sudo() -> GovernanceOf<Test> {
    ResolutionMetadata::new(
        None,
        Some(VoteMetadata::Signal(VoteCall::new(
            OrgRep::Equal(1u64),
            Threshold::new(1u64, None),
            None,
        ))),
    )
    .unwrap()
}

#[test]
fn create_foundation_test() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::create_foundation(
                Origin::signed(1),
                10u32,
                101u64,
                sudo_threshold_no_vote()
            ),
            sp_runtime::DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance")
            }
        );
        assert_noop!(
            Grant::create_foundation(
                Origin::signed(1),
                10u32,
                19u64,
                sudo_threshold_no_vote()
            ),
            Error::<Test>::DepositBelowMinDeposit
        );
        assert!(System::events().is_empty());
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationCreated(1u64, 20u64, 10u32)
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_and_sudo()
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationCreated(2u64, 20u64, 10u32)
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_no_sudo()
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationCreated(3u64, 20u64, 10u32)
        );
    });
}

#[test]
fn donate_2_foundation_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::donate_to_foundation(Origin::signed(2), 1, 10,),
            Error::<Test>::FoundationDNE
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_noop!(
            Grant::donate_to_foundation(Origin::signed(2), 1, 99,),
            sp_runtime::DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance")
            }
        );
        assert_noop!(
            Grant::donate_to_foundation(Origin::signed(2), 1, 9,),
            Error::<Test>::ContributionBelowMinContribution
        );
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationCreated(1u64, 20u64, 10u32)
        );
        assert_ok!(Grant::donate_to_foundation(Origin::signed(2), 1, 10,));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationDonation(2, 10, 1, 30)
        );
    });
}

#[test]
fn submit_application_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::submit_application(
                Origin::signed(1),
                1u64,
                11u32,
                Recipient::new(1, None),
                2u64,
            ),
            Error::<Test>::FoundationDNE
        );
        assert!(System::events().is_empty());
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(1),
            1u64,
            11u32,
            Recipient::new(1, None),
            2u64,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationSubmitted(
                1,
                1,
                Recipient::new(1, None),
                2u64,
                11u32
            )
        );
    });
}

#[test]
fn trigger_app_review_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::trigger_application_review(Origin::signed(1), 1,),
            Error::<Test>::ApplicationDNE
        );
        assert!(System::events().is_empty());
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(1),
            1u64,
            11u32,
            Recipient::new(1, None),
            2u64,
        ));
        assert_noop!(
            Grant::trigger_application_review(Origin::signed(2), 1,),
            Error::<Test>::NotAuthorizedToTriggerApplicationReview
        );
        // even the sudo cannot trigger review for this type of application
        assert_noop!(
            Grant::trigger_application_review(Origin::signed(1), 1,),
            Error::<Test>::NotAuthorizedToTriggerApplicationReview
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_and_sudo()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            2u64,
            11u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_noop!(
            Grant::trigger_application_review(Origin::signed(77), 2,),
            Error::<Test>::NotAuthorizedToTriggerApplicationReview
        );
        assert_ok!(Grant::trigger_application_review(Origin::signed(1), 2,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationReviewTriggered(2, 2, 1)
        );
        assert_ok!(Grant::submit_application(
            Origin::signed(1),
            2u64,
            12u32,
            Recipient::new(1, None),
            7u64,
        ));
        assert_ok!(Grant::trigger_application_review(Origin::signed(2), 3,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationReviewTriggered(2, 3, 2)
        );
    });
}

#[test]
fn approve_reject_application_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::approve_application(Origin::signed(1), 1,),
            Error::<Test>::ApplicationDNE
        );
        assert_noop!(
            Grant::reject_application(Origin::signed(1), 1,),
            Error::<Test>::ApplicationDNE
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(1),
            1u64,
            11u32,
            Recipient::new(1, None),
            2u64,
        ));
        assert_noop!(
            Grant::reject_application(Origin::signed(2), 1,),
            Error::<Test>::NotAuthorizedToRejectApplication
        );
        assert_noop!(
            Grant::approve_application(Origin::signed(77), 1,),
            Error::<Test>::NotAuthorizedToApproveApplication
        );
        assert_ok!(Grant::reject_application(Origin::signed(1), 1,));
        assert_eq!(get_last_event(), RawEvent::ApplicationRejected(1, 1));
        assert_noop!(
            Grant::approve_application(Origin::signed(1), 1,),
            Error::<Test>::ApplicationDNE
        );
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            1u64,
            12u32,
            Recipient::new(2, None),
            9u64,
        ));
        assert_ok!(Grant::approve_application(Origin::signed(1), 2,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationApproved(1, 2, 12u32)
        );
        assert_noop!(
            Grant::reject_application(Origin::signed(1), 2,),
            Error::<Test>::ApplicationNotInValidStateToReject
        );
        assert_noop!(
            Grant::approve_application(Origin::signed(1), 2,),
            Error::<Test>::ApplicationNotInValidStateToApprove
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_and_sudo(),
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            2u64,
            13u32,
            Recipient::new(2, None),
            4u64,
        ));
        assert_ok!(Grant::reject_application(Origin::signed(1), 3,));
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_no_sudo(),
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            3u64,
            14u32,
            Recipient::new(2, None),
            6u64,
        ));
        // bc governance does not have sudo
        assert_noop!(
            Grant::reject_application(Origin::signed(1), 4,),
            Error::<Test>::NotAuthorizedToRejectApplication
        );
    });
}

#[test]
fn submit_milestone_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            1u64,
            11u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_noop!(
            Grant::submit_milestone(
                Origin::signed(2),
                1,
                1,
                12u32,
                Recipient::new(2, None),
                5u64,
            ),
            Error::<Test>::ApplicationMustBeApprovedToSubmitMilestone
        );
        assert_ok!(Grant::approve_application(Origin::signed(1), 1,));
        assert_noop!(
            Grant::submit_milestone(
                Origin::signed(2),
                66u64,
                1,
                12u32,
                Recipient::new(2, None),
                5u64,
            ),
            Error::<Test>::FoundationDNE
        );
        assert_noop!(
            Grant::submit_milestone(
                Origin::signed(2),
                1,
                2,
                12u32,
                Recipient::new(2, None),
                5u64,
            ),
            Error::<Test>::ApplicationDNE
        );
        assert_ok!(Grant::submit_milestone(
            Origin::signed(2),
            1,
            1,
            12u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneSubmitted(
                1,
                1,
                1,
                Recipient::new(2, None),
                5u64,
                12u32
            )
        );
    });
}

#[test]
fn trigger_milestone_review_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::trigger_milestone_review(Origin::signed(1), 1, 1,),
            Error::<Test>::MilestoneDNE
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            1u64,
            11u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_ok!(Grant::approve_application(Origin::signed(1), 1,));
        assert_ok!(Grant::submit_milestone(
            Origin::signed(2),
            1,
            1,
            12u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_noop!(
            Grant::trigger_milestone_review(Origin::signed(1), 1, 1,),
            Error::<Test>::NotAuthorizedToTriggerMilestoneReview
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            new_min_threshold_and_sudo()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            2u64,
            17u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_ok!(Grant::approve_application(Origin::signed(1), 2,));
        assert_ok!(Grant::submit_milestone(
            Origin::signed(2),
            2,
            2,
            19u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_ok!(Grant::trigger_milestone_review(Origin::signed(1), 2, 1,));
        assert_noop!(
            Grant::trigger_milestone_review(Origin::signed(1), 2, 1,),
            Error::<Test>::MilestoneNotInValidStateToTriggerReview
        );
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneReviewTriggered(2, 2, 1, 1,)
        );
    });
}

#[test]
fn approve_reject_milestone_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Grant::approve_milestone(Origin::signed(1), 1, 1,),
            Error::<Test>::MilestoneDNE
        );
        assert_noop!(
            Grant::reject_milestone(Origin::signed(1), 1, 1,),
            Error::<Test>::MilestoneDNE
        );
        assert_ok!(Grant::create_foundation(
            Origin::signed(1),
            10u32,
            20u64,
            sudo_threshold_no_vote()
        ));
        assert_ok!(Grant::submit_application(
            Origin::signed(2),
            1u64,
            11u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_ok!(Grant::approve_application(Origin::signed(1), 1,));
        assert_ok!(Grant::submit_milestone(
            Origin::signed(2),
            1,
            1,
            12u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_noop!(
            Grant::approve_milestone(Origin::signed(66), 1, 1,),
            Error::<Test>::NotAuthorizedToApproveMilestone
        );
        assert_noop!(
            Grant::reject_milestone(Origin::signed(66), 1, 1,),
            Error::<Test>::NotAuthorizedToRejectMilestone
        );
        assert_ok!(Grant::approve_milestone(Origin::signed(1), 1, 1,));
        assert_noop!(
            Grant::approve_milestone(Origin::signed(1), 1, 1,),
            Error::<Test>::MilestoneNotInValidStateToApprove
        );
        assert_noop!(
            Grant::reject_milestone(Origin::signed(1), 1, 1,),
            Error::<Test>::MilestoneNotInValidStateToReject
        );
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneApproved(1, 1, 1, 12u32)
        );
        assert_ok!(Grant::submit_milestone(
            Origin::signed(2),
            1,
            1,
            12u32,
            Recipient::new(2, None),
            5u64,
        ));
        assert_ok!(Grant::reject_milestone(Origin::signed(1), 1, 2,));
        assert_eq!(get_last_event(), RawEvent::MilestoneRejected(1, 1, 2));
    });
}
