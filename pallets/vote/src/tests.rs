use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
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
}
parameter_types! {
    pub const ReservationLimit: u32 = 10000;
}
impl org::Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32;
    type OrgId = u64;
    type Shares = u64;
    type ReservationLimit = ReservationLimit;
}
impl Trait for Test {
    type Event = TestEvent;
    type VoteId = u64;
    type Signal = u64;
}

mod vote {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        org<T>,
        vote<T>,
    }
}
pub type System = system::Module<Test>;
// pub type Organization = org::Module<Test>;
pub type VoteThreshold = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::vote(inner) = e {
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

#[test]
fn vote_creation_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let twentytwo = Origin::signed(22);
        assert_noop!(
            VoteThreshold::create_threshold_approval_vote(twentytwo, None, 1, 4, None, None),
            Error::<Test>::NotAuthorizedToCreateVoteForOrganization
        );
        assert_noop!(
            VoteThreshold::create_threshold_approval_vote(one.clone(), None, 1, 4, Some(3), None),
            Error::<Test>::ThresholdInputDoesNotSatisfySupportGEQTurnoutNorms
        );
        assert_ok!(VoteThreshold::create_threshold_approval_vote(
            one.clone(),
            None,
            1,
            4,
            Some(5),
            None
        ));
        assert_eq!(get_last_event(), RawEvent::NewVoteStarted(1, 1, 1));
    });
}

#[test]
fn vote_threshold_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        // unanimous consent
        assert_ok!(VoteThreshold::create_unanimous_consent_approval_vote(
            one.clone(),
            None,
            1,
            None,
        ));
        for i in 1u64..6u64 {
            let i_origin = Origin::signed(i);
            assert_ok!(VoteThreshold::submit_vote(
                i_origin,
                1,
                VoterView::InFavor,
                None
            ));
        }
        // check that the vote has not passed
        let outcome_almost_passed = VoteThreshold::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        let six = Origin::signed(6);
        assert_ok!(VoteThreshold::submit_vote(six, 1, VoterView::InFavor, None));
        // check that the vote has passed
        let outcome_has_passed = VoteThreshold::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_has_passed, VoteOutcome::Approved);
    });
}
