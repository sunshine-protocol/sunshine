#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

// type aliases
pub type AccountId = u64;
type OrgId = u32;
type FlatShareId = u32;
type WeightedShareId = u32;
pub type Shares = u64;
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
        membership<T>,
        shares_membership<T>,
        shares_atomic<T>,
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
impl membership::Trait for TestRuntime {
    type Event = TestEvent;
}
impl shares_membership::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
}
impl shares_atomic::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
    type Shares = Shares;
    type ReservationLimit = ReservationLimit;
}
impl Trait for TestRuntime {
    type Event = TestEvent;
    type OrgId = OrgId;
    type FlatShareId = FlatShareId;
    type WeightedShareId = WeightedShareId;
    type OrgData = OrgMembership;
    type FlatShareData = FlatShareData;
    type WeightedShareData = WeightedShareData;
}
pub type System = system::Module<TestRuntime>;
pub type OrgMembership = membership::Module<TestRuntime>;
pub type FlatShareData = shares_membership::Module<TestRuntime>;
pub type WeightedShareData = shares_atomic::Module<TestRuntime>;
pub type Org = Module<TestRuntime>;

fn get_last_event() -> RawEvent<u64> {
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
    membership::GenesisConfig::<TestRuntime> {
        omnipotent_key: 1,
        membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    shares_membership::GenesisConfig::<TestRuntime> {
        share_supervisors: None,
        shareholder_membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    shares_atomic::GenesisConfig::<TestRuntime> {
        share_supervisors: None,
        shareholder_membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    GenesisConfig::<TestRuntime> {
        first_organization_supervisor: 1,
        first_organization_value_constitution: b"build cool shit".to_vec(),
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
        let constitution = b"build cool shit".to_vec();
        let expected_organization = Organization::new(ShareID::Flat(1u32), constitution.clone());
        let org_in_storage = Org::organization_states(1u32).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        // check membership from membership module
        for i in 1u64..7u64 {
            assert!(OrgMembership::is_member_of_group(1u32, &i));
        }
        // I guess the events are empty at genesis despite our use of the module's runtime methods for build() in extra genesis
        assert!(System::events().is_empty());
    });
}

#[test]
fn organization_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let accounts = vec![1, 2, 3, 9, 10];
        let constitution: &[u8] = b"no talking about fight club";
        // next line is registration call
        assert_ok!(Org::register_organization_from_accounts(
            one.clone(),
            constitution.clone().to_vec(),
            accounts,
            Some(1),
        ));
        // check organization count changed as expected
        assert_eq!(Org::organization_counter(), 2);
        // Event Emittance in Tests Consistently Fails -- this mystery needs to be solved in order to test...
        assert_eq!(
            get_last_event(),
            RawEvent::NewOrganizationRegistered(1, 2, ShareID::Flat(1), constitution.to_vec()),
        );
        let third_org_accounts = vec![1, 2, 3, 9, 10];
        let third_org_constitution: &[u8] = b"no talking about fight club";
        // next line is registration call
        assert_ok!(Org::register_organization_from_accounts(
            one.clone(),
            third_org_constitution.clone().to_vec(),
            third_org_accounts,
            Some(1),
        ));
        // check organization count changed as expected
        assert_eq!(Org::organization_counter(), 3);
        // Event Emittance in Tests Consistently Fails -- this mystery needs to be solved in order to test...
        assert_eq!(
            get_last_event(),
            RawEvent::NewOrganizationRegistered(
                1,
                3,
                ShareID::Flat(1),
                third_org_constitution.to_vec()
            ),
        );
    });
}

#[test]
fn flat_inner_share_registration_in_organization() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let accounts = vec![1, 2, 3, 9, 10];
        assert_ok!(Org::register_inner_flat_share_group_for_organization(
            one.clone(),
            1u32,
            accounts
        ));
        // check if the share group was registered
        assert_eq!(
            get_last_event(),
            RawEvent::FlatInnerShareGroupAddedToOrg(1, 1, ShareID::Flat(2)),
        );
        let second_share_group = vec![1, 2, 3, 9, 10, 11, 12, 13, 14];
        assert_ok!(Org::register_inner_flat_share_group_for_organization(
            one.clone(),
            1u32,
            second_share_group
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FlatInnerShareGroupAddedToOrg(1, 1, ShareID::Flat(3)),
        );
        let third_share_group = vec![1, 2, 3, 9, 10, 11, 12, 13, 14, 17, 18, 19, 30];
        assert_ok!(Org::register_inner_flat_share_group_for_organization(
            one,
            1u32,
            third_share_group
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FlatInnerShareGroupAddedToOrg(1, 1, ShareID::Flat(4)),
        );
        // check these share groups existence
        for i in 1u32..5u32 {
            assert!(Org::organization_inner_shares(1, ShareID::Flat(i)));
        }
        // check that some members are in each group as expected
        let two_prefix = UUID2::new(1u32, 2u32);
        let three_prefix = UUID2::new(1u32, 3u32);
        let four_prefix = UUID2::new(1u32, 4u32);
        assert!(FlatShareData::is_member_of_group(two_prefix, &10u64));
        assert!(FlatShareData::is_member_of_group(three_prefix, &14u64));
        assert!(FlatShareData::is_member_of_group(four_prefix, &30u64));
    });
}

#[test]
fn weighted_inner_share_registration_for_organization() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let weighted_accounts = vec![(1, 10), (2, 10), (3, 20), (9, 20), (10, 40)];
        assert_ok!(Org::register_inner_weighted_share_group_for_organization(
            one.clone(),
            1u32,
            weighted_accounts
        ));
        // check if the share group was registered
        assert_eq!(
            get_last_event(),
            RawEvent::WeightedInnerShareGroupAddedToOrg(1, 1, ShareID::WeightedAtomic(1)),
        );
        let second_weighted_accounts = vec![(16, 10), (23, 10), (42, 20), (99, 20), (101, 40)];
        assert_ok!(Org::register_inner_weighted_share_group_for_organization(
            one.clone(),
            1u32,
            second_weighted_accounts
        ));
        // check if the share group was registered
        assert_eq!(
            get_last_event(),
            RawEvent::WeightedInnerShareGroupAddedToOrg(1, 1, ShareID::WeightedAtomic(2)),
        );
        let third_weighted_accounts =
            vec![(12, 10), (19, 10), (73, 20), (77, 20), (79, 40), (81, 100)];
        assert_ok!(Org::register_inner_weighted_share_group_for_organization(
            one.clone(),
            1u32,
            third_weighted_accounts
        ));
        // check if the share group was registered
        assert_eq!(
            get_last_event(),
            RawEvent::WeightedInnerShareGroupAddedToOrg(1, 1, ShareID::WeightedAtomic(3)),
        );
        let fourth_weighted_accounts = vec![(1, 10), (2, 10), (3, 20), (4, 20), (5, 40), (6, 100)];
        assert_ok!(Org::register_inner_weighted_share_group_for_organization(
            one.clone(),
            1u32,
            fourth_weighted_accounts
        ));
        // check if the share group was registered
        assert_eq!(
            get_last_event(),
            RawEvent::WeightedInnerShareGroupAddedToOrg(1, 1, ShareID::WeightedAtomic(4)),
        );
        // check that some members are in each group as expected
        assert_eq!(
            WeightedShareData::get_share_profile(1u32, 1u32, &9u64).unwrap(),
            20u64
        );
        assert_eq!(
            WeightedShareData::get_share_profile(1u32, 2u32, &101u64).unwrap(),
            40u64
        );
        assert_eq!(
            WeightedShareData::get_share_profile(1u32, 3u32, &73u64).unwrap(),
            20u64
        );
        assert_eq!(
            WeightedShareData::get_share_profile(1u32, 4u32, &6u64).unwrap(),
            100u64
        );
    });
} // TODO: weighted_outer_share_registration (I assume it works because the other two do and it's the same logic)
