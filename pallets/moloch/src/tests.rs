use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    weights::Weight,
};
use frame_system::{self as system,};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::IdentityLookup,
    Perbill,
    Permill,
};
use util::{
    organization::{
        OrgRep,
        Organization,
    },
    traits::GroupMembership,
    vote::{
        Threshold,
        ThresholdConfig,
        VoterView,
        XorThreshold,
    },
};

// type aliases
pub type AccountId = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for Test where system = frame_system {}
}

mod bank {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for Test {
        system<T>,
        pallet_balances<T>,
        org<T>,
        vote<T>,
        donate<T>,
        bank<T>,
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
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
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
    type WeightInfo = ();
}
impl org::Trait for Test {
    type Event = TestEvent;
    type Cid = u32;
    type OrgId = u64;
    type Shares = u64;
}
impl vote::Trait for Test {
    type Event = TestEvent;
    type VoteId = u64;
    type Signal = u64;
    type ThresholdId = u64;
}
impl donate::Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
}
parameter_types! {
    pub const BigBank: ModuleId = ModuleId(*b"big/bank");
    pub const MinDeposit: u64 = 20;
}
impl Trait for Test {
    type Event = TestEvent;
    type Currency = Balances;
    type BigBank = BigBank;
    type BankId = u64;
    type SpendId = u64;
    type ProposalId = u64;
    type MinDeposit = MinDeposit;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Vote = vote::Module<Test>;
pub type Bank = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64, u64, u64, u64, u64, u64> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::bank(inner) = e {
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
        balances: vec![
            (1, 100),
            (2, 98),
            (3, 200),
            (4, 75),
            (5, 10),
            (6, 69),
            (7, 77),
        ],
    }
    .assimilate_storage(&mut t)
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
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(Org::organization_counter(), 1);
        let constitution = 1738;
        let expected_organization = Organization::new(Some(1), 1, constitution);
        let org_in_storage = Org::organization_states(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

#[test]
fn summon_works() {
    new_test_ext().execute_with(|| {
        let threshold = ThresholdConfig::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_noop!(
            Bank::summon(Origin::signed(1), 1, 19, None, threshold.clone()),
            Error::<Test>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum
        );
        assert_noop!(
            Bank::summon(Origin::signed(5), 1, 21, None, threshold.clone()),
            Error::<Test>::InsufficientBalanceToFundBankOpen
        );
        assert_noop!(
            Bank::summon(Origin::signed(70), 1, 21, None, threshold.clone()),
            Error::<Test>::NotPermittedToOpenBankAccountForOrg
        );
        let false_threshold = ThresholdConfig::new(
            OrgRep::Equal(2),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_noop!(
            Bank::summon(Origin::signed(1), 1, 20, None, false_threshold),
            Error::<Test>::ThresholdCannotBeSetForOrg
        );
        assert_ok!(Bank::summon(Origin::signed(1), 1, 20, None, threshold));
        let expected_event = RawEvent::BankAccountOpened(1, 1, 20, 1, None);
        assert_eq!(get_last_event(), expected_event);
    });
}

#[test]
fn propose_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bank::propose_spend(Origin::signed(1), 1, 19, 1),
            Error::<Test>::BankMustExistToProposeFrom
        );
        let threshold = ThresholdConfig::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Bank::summon(Origin::signed(1), 1, 20, None, threshold),);
        assert_noop!(
            Bank::propose_spend(Origin::signed(7), 1, 10, 7),
            Error::<Test>::MustBeMemberToSponsorProposal
        );
        assert_noop!(
            Bank::propose_member(Origin::signed(7), 1, 0, 100, 7),
            Error::<Test>::MustBeMemberToSponsorProposal
        );
        assert_ok!(Bank::propose_spend(Origin::signed(1), 1, 10, 7),);
        let expected_event = RawEvent::SpendProposedByMember(1, 1, 1, 10, 7);
        assert_eq!(get_last_event(), expected_event);
        assert_ok!(Bank::propose_member(Origin::signed(1), 1, 10, 5, 7),);
        let expected_event = RawEvent::NewMemberProposal(1, 1, 1, 10, 5, 7);
        assert_eq!(get_last_event(), expected_event);
    });
}

#[test]
fn trigger_vote_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bank::trigger_vote_on_spend_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfBaseBankDNE
        );
        assert_noop!(
            Bank::trigger_vote_on_member_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfBaseBankDNE
        );
        let threshold = ThresholdConfig::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Bank::summon(Origin::signed(1), 1, 20, None, threshold),);
        assert_noop!(
            Bank::trigger_vote_on_spend_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_noop!(
            Bank::trigger_vote_on_member_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_ok!(Bank::propose_spend(Origin::signed(1), 1, 10, 7),);
        assert_noop!(
            Bank::trigger_vote_on_spend_proposal(Origin::signed(7), 1, 1),
            Error::<Test>::NotPermittedToTriggerVoteForBankAccount
        );
        assert_ok!(Bank::trigger_vote_on_spend_proposal(
            Origin::signed(1),
            1,
            1
        ));
        let expected_event = RawEvent::VoteTriggeredOnSpendProposal(1, 1, 1, 1);
        assert_eq!(expected_event, get_last_event());
        assert_noop!(
            Bank::trigger_vote_on_member_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_ok!(Bank::propose_member(Origin::signed(1), 1, 10, 5, 7),);
        assert_noop!(
            Bank::trigger_vote_on_member_proposal(Origin::signed(7), 1, 1),
            Error::<Test>::NotPermittedToTriggerVoteForBankAccount
        );
        assert_ok!(Bank::trigger_vote_on_member_proposal(
            Origin::signed(1),
            1,
            1
        ));
        let expected_event =
            RawEvent::VoteTriggeredOnMemberProposal(1, 1, 1, 2);
        assert_eq!(expected_event, get_last_event());
    });
}
