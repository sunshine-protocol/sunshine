use super::*;
use frame_support::{assert_err, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

// type aliases
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
    type IpfsReference = u32; // TODO: replace with utils_identity::Cid
    type OrgId = u64;
    type Shares = u64;
    type ReservationLimit = ReservationLimit;
}
impl Trait for Test {
    type Event = TestEvent;
    type PetitionId = u64;
}

mod vote_petition {
    pub use crate::Event;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        org<T>,
        vote_petition<T>,
    }
}
pub type System = system::Module<Test>;
pub type VotePetition = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u32, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::vote_petition(inner) = e {
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
fn create_petition_happy_path() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let ten = Origin::signed(10);
        let new_topic = 22;
        assert_eq!(VotePetition::petition_id_nonce(), 0);
        // auth only allows sudo or organization supervisor
        assert_err!(
            VotePetition::create_petition(ten, 1, Some(new_topic.clone()), 4, 2, None,),
            Error::<Test>::NotAuthorizedToCreatePetition
        );
        assert_eq!(VotePetition::petition_id_nonce(), 0);
        assert_ok!(VotePetition::create_petition(
            one,
            1,
            Some(new_topic),
            4,
            2,
            None,
        ));
        assert_eq!(VotePetition::petition_id_nonce(), 1);
        assert_eq!(get_last_event(), RawEvent::NewPetitionStarted(1, 1, 1));
        let new_petition_state = PetitionState::new(Some(new_topic), 1, 4, 2, 6, None).unwrap();
        assert_eq!(
            new_petition_state,
            VotePetition::petition_states(1).unwrap()
        );
    });
}
