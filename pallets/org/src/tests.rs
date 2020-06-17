#![cfg(test)]

use super::*;
use frame_support::assert_ok;
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

mod org {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        system<T>,
        org<T>,
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
impl Trait for TestRuntime {
    type Event = TestEvent;
    type IpfsReference = u32; // TODO: replace with utils_identity::Cid
    type OrgId = u64;
    type Shares = u64;
    type ReservationLimit = ReservationLimit;
}
pub type System = system::Module<TestRuntime>;
pub type Org = Module<TestRuntime>;

fn get_last_event() -> RawEvent<u64, u64, u64, u32> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::org(inner) = e {
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
        .build_storage::<TestRuntime>()
        .unwrap();
    GenesisConfig::<TestRuntime> {
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
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Org::organization_counter(), 1);
        let constitution = 1738;
        let expected_organization = Organization::new(Some(1), None, constitution);
        let org_in_storage = Org::organization_states(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

#[test]
fn organization_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let accounts = vec![1, 2, 3, 9, 10];
        let constitution = 1110011;
        assert_ok!(Org::register_flat_org(
            one.clone(),
            Some(1),
            None,
            constitution,
            accounts,
        ));
        assert_eq!(Org::organization_counter(), 2);
        assert_eq!(
            get_last_event(),
            RawEvent::NewFlatOrganizationRegistered(1, 2, constitution, 5),
        );
        let third_org_accounts = vec![(1, 10), (2, 10), (3, 10), (9, 10), (10, 10)];
        let third_org_constitution = 9669;
        assert_ok!(Org::register_weighted_org(
            one.clone(),
            Some(1),
            None,
            third_org_constitution,
            third_org_accounts,
        ));
        assert_eq!(Org::organization_counter(), 3);
        assert_eq!(
            get_last_event(),
            RawEvent::NewWeightedOrganizationRegistered(1, 3, third_org_constitution, 50,),
        );
    });
}

// #[test]
// fn share_reservation() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         assert_ok!(Org::reserve_shares(one.clone(), 1, 1, 1));
//         let profile = Org::members(1, 1).unwrap();
//         let first_times_reserved = profile.times_reserved();
//         // // check that method calculates correctly
//         assert_eq!(first_times_reserved, 1);
//         assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
//         let second_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let second_times_reserved = second_profile.times_reserved();
//         assert_eq!(second_times_reserved, 2);
//         let mut n = 0u32;
//         while n < 20 {
//             assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
//             n += 1;
//         }
//         let n_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let n_times_reserved = n_profile.times_reserved();
//         assert_eq!(n_times_reserved, 22);

//         // check same logic with another member of the first group
//         assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
//         let a_profile = AtomicShares::profile(prefix_key, 2).unwrap();
//         let a_first_times_reserved = a_profile.times_reserved();
//         // // check that method calculates correctly
//         assert_eq!(a_first_times_reserved, 1);
//         assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
//         let a_second_profile = AtomicShares::profile(prefix_key, 2).unwrap();
//         let a_second_times_reserved = a_second_profile.times_reserved();
//         assert_eq!(a_second_times_reserved, 2);
//         let mut a_n = 0u32;
//         while a_n < 20 {
//             assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
//             a_n += 1;
//         }
//         let a_n_profile = AtomicShares::profile(prefix_key, 2).unwrap();
//         let a_n_times_reserved = a_n_profile.times_reserved();
//         assert_eq!(a_n_times_reserved, 22);
//     });
// }

// #[test]
// fn share_unreservation() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
//         let prefix_key = UUID2::new(1, 1);
//         let profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let first_times_reserved = profile.times_reserved();
//         // // check that method calculates correctly
//         assert_eq!(first_times_reserved, 1);
//         assert_ok!(AtomicShares::unreserve_shares(one.clone(), 1, 1, 1));
//         let un_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let first_times_un_reserved = un_profile.times_reserved();
//         // // check that method calculates correctly
//         assert_eq!(first_times_un_reserved, 0);
//     });
// }

// #[test]
// fn share_lock() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         let prefix_key = UUID2::new(1, 1);
//         let profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let unlocked = profile.is_unlocked();
//         assert_eq!(unlocked, true);
//         assert_ok!(AtomicShares::lock_shares(one.clone(), 1, 1, 1));
//         let locked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let locked = !locked_profile.is_unlocked();
//         assert_eq!(locked, true);
//     });
// }

// #[test]
// fn share_unlock() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         let prefix_key = UUID2::new(1, 1);
//         let profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let unlocked = profile.is_unlocked();
//         assert_eq!(unlocked, true);
//         assert_ok!(AtomicShares::lock_shares(one.clone(), 1, 1, 1));
//         let locked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let locked = !locked_profile.is_unlocked();
//         assert_eq!(locked, true);
//         assert_ok!(AtomicShares::unlock_shares(one.clone(), 1, 1, 1));
//         let unlocked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
//         let is_unlocked = unlocked_profile.is_unlocked();
//         assert_eq!(is_unlocked, true);
//     });
// }

// #[test]
// fn share_issuance() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         let prefix_key = UUID2::new(1, 1);
//         let pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
//         let pre_shares = pre_profile.total();

//         assert_eq!(pre_shares, 10);
//         // issue 10 new shares to 7
//         assert_ok!(AtomicShares::issue_shares(one.clone(), 1, 1, 10, 10));

//         let post_profile = AtomicShares::profile(prefix_key, 10).unwrap();
//         let post_shares = post_profile.total();

//         assert_eq!(post_shares, 20);
//     });
// }

// #[test]
// fn share_burn() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         let prefix_key = UUID2::new(1, 1);
//         let pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
//         let pre_shares = pre_profile.total();

//         assert_eq!(pre_shares, 10);
//         // issue 10 new shares to 10
//         assert_ok!(AtomicShares::issue_shares(one.clone(), 1, 1, 10, 10));

//         let pre_pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
//         let pre_pre_shares = pre_pre_profile.total();

//         assert_eq!(pre_pre_shares, 20);
//         // burn 10 new shares for 10
//         assert_ok!(AtomicShares::burn_shares(one.clone(), 1, 1, 10, 10));
//         let post_profile = AtomicShares::profile(prefix_key, 10).unwrap();
//         let post_shares = post_profile.total();

//         assert_eq!(post_shares, 10);
//     });
// }
