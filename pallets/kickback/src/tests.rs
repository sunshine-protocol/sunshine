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

mod kickback {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        pallet_balances<T>,
        kickback<T>,
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
parameter_types! {
    pub const EventPool: ModuleId = ModuleId(*b"event/id");
    pub const MaxAttendance: u32 = 100;
    pub const MinReservationReq: u64 = 5;
}
impl Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32;
    type Currency = Balances;
    type KickbackEventId = u64;
    type EventPool = EventPool;
    type MinReservationReq = MinReservationReq;
    type MaxAttendance = MaxAttendance;
}
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Kickback = Module<Test>;

fn get_last_event() -> RawEvent<u64, u32, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::kickback(inner) = e {
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
fn post_kickback_event_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Kickback::post_kickback_event(Origin::signed(1), 5u32, 4, 10,),
            Error::<Test>::EventReservationReqBelowModuleMin
        );
        assert_noop!(
            Kickback::post_kickback_event(Origin::signed(1), 5u32, 7, 101,),
            Error::<Test>::EventAttendanceLimitAboveModuleMax
        );
        assert_ok!(Kickback::post_kickback_event(
            Origin::signed(1),
            5u32,
            7,
            90,
        ));
        let expected_event = RawEvent::EventPosted(1, 7, 1, 5);
        assert_eq!(expected_event, get_last_event());
    });
}

#[test]
fn reserve_seat_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Kickback::reserve_seat(Origin::signed(2), 1,),
            Error::<Test>::KickbackEventDNE
        );
        assert_ok!(Kickback::post_kickback_event(
            Origin::signed(1),
            5u32,
            7,
            2,
        ));
        assert_ok!(Kickback::reserve_seat(Origin::signed(2), 1,),);
        assert_noop!(
            Kickback::reserve_seat(Origin::signed(2), 1,),
            Error::<Test>::AlreadyMadeReservation
        );
        assert_ok!(Kickback::reserve_seat(Origin::signed(3), 1,),);
        assert_noop!(
            Kickback::reserve_seat(Origin::signed(4), 1,),
            Error::<Test>::AttendanceLimitReached
        );
        let expected_event = RawEvent::EventSeatReserved(1, 3);
        assert_eq!(expected_event, get_last_event());
    });
}

#[test]
fn publish_attendance_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Kickback::publish_attendance_and_execute_redistribution(
                Origin::signed(1),
                1,
                vec![2, 3],
            ),
            Error::<Test>::KickbackEventDNE
        );
        assert_ok!(Kickback::post_kickback_event(
            Origin::signed(1),
            5u32,
            7,
            2,
        ));
        assert_noop!(
            Kickback::publish_attendance_and_execute_redistribution(
                Origin::signed(1),
                1,
                vec![2, 3],
            ),
            Error::<Test>::AttendanceMustBeGreaterThanZero
        );
        assert_ok!(Kickback::reserve_seat(Origin::signed(2), 1,),);
        assert_noop!(
            Kickback::publish_attendance_and_execute_redistribution(
                Origin::signed(2),
                1,
                vec![2, 3],
            ),
            Error::<Test>::NotAuthorizedToPublishAttendance
        );
        assert_ok!(Kickback::reserve_seat(Origin::signed(3), 1,),);
        assert_noop!(
            Kickback::publish_attendance_and_execute_redistribution(
                Origin::signed(1),
                1,
                vec![],
            ),
            Error::<Test>::AttendanceMustBeGreaterThanZero
        );
        assert_ok!(Kickback::publish_attendance_and_execute_redistribution(
            Origin::signed(1),
            1,
            vec![2],
        ),);
    });
}
