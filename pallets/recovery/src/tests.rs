#![cfg(test)]

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
    traits::{
        BlakeTwo256,
        IdentityLookup,
    },
    Perbill,
};

pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

mod recovery {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        frame_system<T>,
        pallet_balances<T>,
        recovery<T>,
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
    type Hashing = BlakeTwo256;
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
impl pallet_balances::Trait for TestRuntime {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type MaxLocks = MaxLocks;
    type AccountStore = System;
    type WeightInfo = ();
}
parameter_types! {
    pub const Pool: ModuleId = ModuleId(*b"recovery");
}
impl Trait for TestRuntime {
    type Event = TestEvent;
    type SecretId = u64;
    type RoundId = u64;
    type Pool = Pool;
    type Currency = Balances;
}
pub type System = frame_system::Module<TestRuntime>;
pub type Balances = pallet_balances::Module<TestRuntime>;
pub type Recovery = recovery::Module<TestRuntime>;

fn get_last_event() -> RawEvent<u64, H256, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::recovery(inner) = e {
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
    pallet_balances::GenesisConfig::<TestRuntime> {
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
fn invite_group_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Recovery::invite_group(
                Origin::signed(1),
                101,
                5,
                vec![1, 2, 3, 4, 5, 6]
            ),
            Error::<TestRuntime>::UserCannotAffordRequest
        );
        assert_eq!(Balances::total_balance(&1), 100);
        let secret_account = Recovery::secret_account_id(1);
        assert_eq!(Balances::total_balance(&secret_account), 0);
        assert_ok!(Recovery::invite_group(
            Origin::signed(1),
            20,
            5,
            vec![1, 2, 3, 4, 5, 6]
        ));
        assert_eq!(Balances::total_balance(&1), 80);
        assert_eq!(Balances::total_balance(&secret_account), 20);
        let expected_event = RawEvent::SecretGroupInitialized(1, 1);
        assert_eq!(get_last_event(), expected_event);
    });
}

#[test]
fn revoke_invitation_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Recovery::revoke_invitation(Origin::signed(1), 1, 1),
            Error::<TestRuntime>::SecretDNE
        );
        assert_ok!(Recovery::invite_group(
            Origin::signed(1),
            20,
            5,
            vec![1, 2, 3, 4, 5, 6]
        ));
        assert_noop!(
            Recovery::revoke_invitation(Origin::signed(2), 1, 3),
            Error::<TestRuntime>::NotAuthorizedForSecret
        );
        assert_ok!(Recovery::revoke_invitation(Origin::signed(1), 1, 3));
        let expected_event = RawEvent::RevokedInvitation(1, 3);
        assert_eq!(get_last_event(), expected_event);
    });
}

#[test]
fn commit_reveal_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Recovery::commit_hash(Origin::signed(2), 1, H256::random()),
            Error::<TestRuntime>::SecretDNE
        );
        assert_noop!(
            Recovery::reveal_preimage(
                Origin::signed(1),
                1,
                b"good code never dies".to_vec()
            ),
            Error::<TestRuntime>::SecretDNE
        );
        assert_ok!(Recovery::invite_group(
            Origin::signed(1),
            20,
            5,
            vec![1, 2, 3, 4, 5, 6]
        ));
        assert_noop!(
            Recovery::reveal_preimage(
                Origin::signed(2),
                1,
                b"good code never dies".to_vec()
            ),
            Error::<TestRuntime>::OnlyRevealPreimageIfRecoveryRequested
        );
        let hash: H256 = BlakeTwo256::hash(&b"good code never dies"[..]);
        assert_noop!(
            Recovery::commit_hash(Origin::signed(7), 1, H256::random()),
            Error::<TestRuntime>::NotAuthorizedForSecret
        );
        assert_eq!(Balances::free_balance(&2), 98);
        assert_ok!(Recovery::commit_hash(Origin::signed(2), 1, hash));
        assert_eq!(Balances::free_balance(&2), 93);
        let expected_event = RawEvent::CommittedSecretHash(2, 1, 0, hash);
        assert_eq!(get_last_event(), expected_event);
        assert_ok!(Recovery::request_recovery(Origin::signed(1), 1));
        assert_noop!(
            Recovery::reveal_preimage(
                Origin::signed(10),
                1,
                b"good code never dies".to_vec()
            ),
            Error::<TestRuntime>::NotAuthorizedForSecret
        );
        assert_noop!(
            Recovery::reveal_preimage(
                Origin::signed(2),
                1,
                b"expect the unexpected".to_vec()
            ),
            Error::<TestRuntime>::PreimageHashDNEHash
        );
        assert_ok!(Recovery::reveal_preimage(
            Origin::signed(2),
            1,
            b"good code never dies".to_vec()
        ),);
        let expected_event = RawEvent::RevealedPreimage(
            2,
            1,
            0,
            b"good code never dies".to_vec(),
        );
        assert_eq!(get_last_event(), expected_event);
    });
}
