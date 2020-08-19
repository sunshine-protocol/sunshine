use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    weights::Weight,
};
use rand::{
    rngs::OsRng,
    RngCore,
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

mod doc {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        frame_system<T>,
        doc<T>,
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
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
}
impl Trait for Test {
    type Event = TestEvent;
    type CodeId = u64;
    type DocId = u64;
    type Cid = u64;
}
pub type System = frame_system::Module<Test>;
pub type Doc = Module<Test>;

fn random(output_len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; output_len];
    OsRng.fill_bytes(&mut buf);
    buf
}

fn get_last_event() -> RawEvent<u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::doc(inner) = e {
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
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert!(System::events().is_empty());
    });
}

#[test]
fn new_code_set_works() {
    new_test_ext().execute_with(|| {
        let new_code_set = vec![random(10), random(11)];
        let empty_code_set = vec![];
        assert_noop!(
            Doc::new_code_set(Origin::signed(1), empty_code_set,),
            Error::<Test>::MustIncludeCodeToCreateNewCodeSet
        );
        assert_ok!(Doc::new_code_set(Origin::signed(1), new_code_set,));
        assert_eq!(get_last_event(), RawEvent::NewCodeSet(1));
    });
}

#[test]
fn new_encoded_object_works() {
    new_test_ext().execute_with(|| {
        let first_code = random(10);
        let new_code_set = vec![first_code.clone(), random(11)];
        assert_noop!(
            Doc::new_encoded_object(Origin::signed(1), 1, random(12)),
            Error::<Test>::CodeIdNotRegistered
        );
        assert_ok!(Doc::new_code_set(Origin::signed(1), new_code_set,));
        assert_eq!(get_last_event(), RawEvent::NewCodeSet(1));
        assert_noop!(
            Doc::new_encoded_object(Origin::signed(1), 1, first_code),
            Error::<Test>::CodeAlreadyRegisteredInSet
        );
        assert_ok!(Doc::new_encoded_object(Origin::signed(1), 1, random(12)));
    });
}

#[test]
fn new_doc_works() {
    new_test_ext().execute_with(|| {
        assert_ok!(Doc::new_doc(Origin::signed(1), 10));
        assert_eq!(get_last_event(), RawEvent::NewDoc(1, 10));
    });
}
