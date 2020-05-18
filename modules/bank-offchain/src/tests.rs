use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use util::{organization::Organization, traits::GroupMembership};

// type aliases
pub type AccountId = u64;
pub type Shares = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

mod bank_offchain {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        membership<T>,
        shares_membership<T>,
        shares_atomic<T>,
        org<T>,
        bank_offchain<T>,
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;
parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::one();
    pub const ReservationLimit: u32 = 10000;
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
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for Test {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
}
impl membership::Trait for Test {
    type Event = TestEvent;
}
impl shares_membership::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
}
// impl vote_petition::Trait for Test {
//     type Event = TestEvent;
//     type OrgData = membership::Module<Test>;
//     type ShareData = shares_membership::Module<Test>;
// }
impl shares_atomic::Trait for Test {
    type Event = TestEvent;
    type OrgData = membership::Module<Test>;
    type Shares = Shares;
    type ReservationLimit = ReservationLimit;
}
impl org::Trait for Test {
    type Event = TestEvent;
    type OrgData = OrgMembership;
    type FlatShareData = FlatShareData;
    type WeightedShareData = WeightedShareData;
}
impl Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
    type Organization = OrganizationWrapper;
}
pub type System = frame_system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type OrgMembership = membership::Module<Test>;
pub type FlatShareData = shares_membership::Module<Test>;
pub type WeightedShareData = shares_atomic::Module<Test>;
pub type OrganizationWrapper = org::Module<Test>;
pub type BankOffChain = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::bank_offchain(inner) = e {
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
    membership::GenesisConfig::<Test> {
        omnipotent_key: 1,
        membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    shares_membership::GenesisConfig::<Test> {
        share_supervisors: None,
        shareholder_membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    shares_atomic::GenesisConfig::<Test> {
        share_supervisors: None,
        shareholder_membership: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();
    org::GenesisConfig::<Test> {
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
        assert_eq!(OrganizationWrapper::organization_counter(), 1);
        let constitution = b"build cool shit".to_vec();
        let expected_organization = Organization::new(ShareID::Flat(1u32), constitution.clone());
        let org_in_storage = OrganizationWrapper::organization_states(1u32).unwrap();
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
fn offchain_bank_functionality() {
    new_test_ext().execute_with(|| {
        // check if it works for the first group
        let one = Origin::signed(1);
        assert_ok!(
            BankOffChain::register_offchain_bank_account_for_organization(one.clone(), 1u32)
        );
        assert_eq!(
            get_last_event(),
            RawEvent::NewOffChainTreasuryRegisteredForOrg(1, 1),
        );
        // an account in the org uses it to log a payment
        let six = Origin::signed(6);
        let sixtynine = Origin::signed(69);
        assert_noop!(
            BankOffChain::use_offchain_bank_account_to_claim_payment_sent(
                sixtynine.clone(),
                1,
                69,
                69
            ),
            Error::<Test>::MustBeAMemberToUseOffChainBankAccountToClaimPaymentSent
        );
        assert_ok!(BankOffChain::use_offchain_bank_account_to_claim_payment_sent(six, 1, 69, 69));
        assert_eq!(
            get_last_event(),
            RawEvent::SenderClaimsPaymentSent(1, 6, 69, 69, 0),
        );
        // note how this error is returned because one is NOT the recipient
        assert_noop!(
            BankOffChain::use_offchain_bank_account_to_confirm_payment_received(one, 1, 0, 6, 69),
            Error::<Test>::SenderMustClaimPaymentSentForRecipientToConfirm
        );
        assert_ok!(
            BankOffChain::use_offchain_bank_account_to_confirm_payment_received(
                sixtynine, 1, 0, 6, 69
            )
        );
        assert_eq!(
            get_last_event(),
            RawEvent::RecipientConfirmsPaymentReceived(1, 6, 69, 69, 0),
        );
    });
}
