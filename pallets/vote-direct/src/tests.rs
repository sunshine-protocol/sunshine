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
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
}
impl Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32;
    type VoteId = u64;
    type Signal = u64;
}

mod vote {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        vote<T>,
    }
}
pub type System = system::Module<Test>;
// pub type Organization = org::Module<Test>;
pub type Vote = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64> {
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
    let t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let mut ext: sp_io::TestExternalities = t.into();
    ext.execute_with(|| System::set_block_number(1));
    ext
}

#[test]
fn vote_creation_works() {
    new_test_ext().execute_with(|| {
        let vote_set: SimpleShareGenesis<u64, u64> =
            vec![(1, 10), (2, 20)].into();
        assert_noop!(
            Vote::create_signal_vote(
                Origin::signed(22),
                None,
                vote_set.clone(),
                Threshold::new(31, None),
                None
            ),
            Error::<Test>::InputThresholdExceedsBounds
        );
        assert_ok!(Vote::create_signal_vote(
            Origin::signed(1),
            None,
            vote_set,
            Threshold::new(10, None),
            None
        ));
        assert_eq!(get_last_event(), RawEvent::NewVoteStarted(1, 1));
    });
}

#[test]
fn vote_signal_threshold_works() {
    new_test_ext().execute_with(|| {
        let vote_set: SimpleShareGenesis<u64, u64> =
            vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)].into();
        // unanimous consent
        assert_ok!(Vote::create_signal_vote(
            Origin::signed(1),
            None,
            vote_set,
            Threshold::new(6, None),
            None
        ));
        for i in 1u64..6u64 {
            let i_origin = Origin::signed(i);
            assert_ok!(Vote::submit_vote(
                i_origin,
                1,
                VoterView::InFavor,
                None
            ));
        }
        // check that the vote has not passed
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        let six = Origin::signed(6);
        assert_ok!(Vote::submit_vote(six, 1, VoterView::InFavor, None));
        // check that the vote has passed
        let outcome_has_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_has_passed, VoteOutcome::Approved);
        assert_eq!(get_last_event(), RawEvent::Voted(1, 6, VoterView::InFavor));
    });
}

#[test]
fn vote_pct_threshold_works() {
    new_test_ext().execute_with(|| {
        let vote_set: SimpleShareGenesis<u64, u64> =
            vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)].into();
        // 34% passage requirement => 3 people at least
        assert_ok!(Vote::create_percent_vote(
            Origin::signed(1),
            None,
            vote_set,
            Threshold::new(Permill::from_percent(34), None),
            None,
        ));
        // check that the vote has not passed
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        assert_ok!(Vote::submit_vote(
            Origin::signed(1),
            1,
            VoterView::InFavor,
            None
        ));
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        assert_ok!(Vote::submit_vote(
            Origin::signed(2),
            1,
            VoterView::InFavor,
            None
        ));
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        assert_ok!(Vote::submit_vote(
            Origin::signed(3),
            1,
            VoterView::InFavor,
            None
        ));
        // check that the vote has passed
        let outcome_has_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_has_passed, VoteOutcome::Approved);
        assert_eq!(get_last_event(), RawEvent::Voted(1, 3, VoterView::InFavor));
    });
}

#[test]
fn changing_votes_upholds_invariants() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Vote::submit_vote(Origin::signed(1), 1, VoterView::Against, None),
            Error::<Test>::NoVoteStateForVoteRequest
        );
        let vote_set: SimpleShareGenesis<u64, u64> =
            vec![(1, 1), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)].into();
        // unanimous consent threshold
        assert_ok!(Vote::create_signal_vote(
            Origin::signed(1),
            None,
            vote_set,
            Threshold::new(6, None),
            None,
        ));
        for i in 1u64..6u64 {
            let i_origin = Origin::signed(i);
            assert_ok!(Vote::submit_vote(
                i_origin,
                1,
                VoterView::InFavor,
                None
            ));
        }
        // change the vote of voter 5
        assert_ok!(Vote::submit_vote(
            Origin::signed(5u64),
            1,
            VoterView::Against,
            None
        ));
        // check that the vote has not passed
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Voting);
        // vote in favor for 6
        assert_ok!(Vote::submit_vote(
            Origin::signed(6),
            1,
            VoterView::InFavor,
            None
        ));
        // cannot change vote to NoVote from an existing vote
        assert_noop!(
            Vote::submit_vote(
                Origin::signed(6),
                1,
                VoterView::Uninitialized,
                None
            ),
            Error::<Test>::VoteChangeNotSupported
        );
        // check that the vote has still not passed (even after 6 had voted in favor because 5 had changed their vote)
        let outcome_has_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_has_passed, VoteOutcome::Voting);
        // change the vote of voter 5 back
        assert_ok!(Vote::submit_vote(
            Origin::signed(5u64),
            1,
            VoterView::InFavor,
            None
        ));
        // check that the vote has now passed
        let outcome_almost_passed = Vote::get_vote_outcome(1).unwrap();
        assert_eq!(outcome_almost_passed, VoteOutcome::Approved);
    });
}
