#![cfg(test)]

use super::*;
use frame_support::{
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
    pub enum Origin for TestRuntime {}
}

mod org {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        frame_system<T>,
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
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
}
impl Trait for TestRuntime {
    type Event = TestEvent;
    type Cid = u32;
    type OrgId = u64;
    type Shares = u64;
}
pub type System = frame_system::Module<TestRuntime>;
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
        sudo: 1,
        doc: 1738,
        mems: vec![1, 2, 3, 4, 5, 6],
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

#[test]
fn organization_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let accounts = vec![1, 2, 3, 9, 10];
        let constitution = 1110011;
        assert_ok!(Org::new_flat_org(
            one.clone(),
            Some(1),
            None,
            constitution,
            accounts,
        ));
        assert_eq!(Org::org_counter(), 2);
        assert_eq!(
            get_last_event(),
            RawEvent::NewFlatOrg(1, 2, constitution, 5),
        );
        let third_org_accounts =
            vec![(1, 10), (2, 10), (3, 10), (9, 10), (10, 10)];
        let third_org_constitution = 9669;
        assert_ok!(Org::new_weighted_org(
            one,
            Some(1),
            None,
            third_org_constitution,
            third_org_accounts,
        ));
        assert_eq!(Org::org_counter(), 3);
        assert_eq!(
            get_last_event(),
            RawEvent::NewWeightedOrg(1, 3, third_org_constitution, 50,),
        );
    });
}

#[test]
fn share_lock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let profile = Org::members(1, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(Org::lock_shares(one, 1, 1));
        let locked_profile = Org::members(1, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
    });
}

#[test]
fn share_unlock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let profile = Org::members(1, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(Org::lock_shares(one.clone(), 1, 1));
        let locked_profile = Org::members(1, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
        assert_ok!(Org::unlock_shares(one, 1, 1));
        let unlocked_profile = Org::members(1, 1).unwrap();
        let is_unlocked = unlocked_profile.is_unlocked();
        assert_eq!(is_unlocked, true);
    });
}

#[test]
fn share_issuance() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let pre_profile = Org::members(1, 1).unwrap();
        let pre_shares = pre_profile.total();

        assert_eq!(pre_shares, 1);
        // issue 10 new shares to member 1
        assert_ok!(Org::issue_shares(one, 1, 1, 10));

        let post_profile = Org::members(1, 1).unwrap();
        let post_shares = post_profile.total();

        assert_eq!(post_shares, 11);
    });
}

#[test]
fn share_burn() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let pre_profile = Org::members(1, 1).unwrap();
        let pre_shares = pre_profile.total();

        assert_eq!(pre_shares, 1);
        // issue 10 new shares to 10
        assert_ok!(Org::issue_shares(one.clone(), 1, 1, 10));

        let pre_pre_profile = Org::members(1, 1).unwrap();
        let pre_pre_shares = pre_pre_profile.total();

        assert_eq!(pre_pre_shares, 11);
        // burn 10 new shares for 10
        assert_ok!(Org::burn_shares(one, 1, 1, 5));
        let post_profile = Org::members(1, 1).unwrap();
        let post_shares = post_profile.total();

        assert_eq!(post_shares, 6);
    });
}
