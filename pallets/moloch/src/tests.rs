use super::*;
use frame_support::{
    assert_noop,
    assert_ok,
    impl_outer_event,
    impl_outer_origin,
    parameter_types,
    traits::OnFinalize,
    weights::Weight,
};
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
        OrganizationSource,
    },
    traits::{
        GroupMembership,
        RegisterOrganization,
        ShareInformation,
    },
    vote::{
        Threshold,
        ThresholdInput,
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
        frame_system<T>,
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
    type MemId = u64;
    type MinDeposit = MinDeposit;
}
pub type System = frame_system::Module<Test>;
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

/// Auxiliary method for simulating block time passing
fn run_to_block(n: u64) {
    while System::block_number() < n {
        Bank::on_finalize(System::block_number());
        System::set_block_number(System::block_number() + 1);
    }
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
    GenesisConfig::<Test> {
        spend_poll_frequency: 7,
        member_poll_frequency: 7,
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
fn summon_works() {
    new_test_ext().execute_with(|| {
        let threshold = ThresholdInput::new(
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
        let false_threshold = ThresholdInput::new(
            OrgRep::Equal(2),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_noop!(
            Bank::summon(Origin::signed(1), 1, 20, None, false_threshold),
            Error::<Test>::ThresholdCannotBeSetForOrg
        );
        assert_ok!(Bank::summon(Origin::signed(1), 1, 20, None, threshold));
        let expected_event = RawEvent::AccountOpened(1, 1, 20, 1, None);
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
        let threshold = ThresholdInput::new(
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
        let expected_event = RawEvent::SpendProposed(1, 1, 1, 10, 7);
        assert_eq!(get_last_event(), expected_event);
        assert_ok!(Bank::propose_member(Origin::signed(1), 1, 10, 5, 7),);
        let expected_event = RawEvent::MemberProposed(1, 1, 1, 10, 5, 7);
        assert_eq!(get_last_event(), expected_event);
    });
}

#[test]
fn trigger_vote_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bank::spend_trigger_vote(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfBaseBankDNE
        );
        assert_noop!(
            Bank::member_trigger_vote(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfBaseBankDNE
        );
        let threshold = ThresholdInput::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Bank::summon(Origin::signed(1), 1, 20, None, threshold),);
        assert_noop!(
            Bank::spend_trigger_vote(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_noop!(
            Bank::member_trigger_vote(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_ok!(Bank::propose_spend(Origin::signed(1), 1, 10, 7),);
        assert_noop!(
            Bank::spend_trigger_vote(Origin::signed(7), 1, 1),
            Error::<Test>::NotPermittedToTriggerVoteForBankAccount
        );
        assert_ok!(Bank::spend_trigger_vote(Origin::signed(1), 1, 1));
        let expected_event = RawEvent::SpendVoteTriggered(1, 1, 1, 1);
        assert_eq!(expected_event, get_last_event());
        assert_noop!(
            Bank::member_trigger_vote(Origin::signed(1), 1, 1),
            Error::<Test>::CannotTriggerVoteIfProposalDNE
        );
        assert_ok!(Bank::propose_member(Origin::signed(1), 1, 10, 5, 7),);
        assert_noop!(
            Bank::member_trigger_vote(Origin::signed(7), 1, 1),
            Error::<Test>::NotPermittedToTriggerVoteForBankAccount
        );
        assert_ok!(Bank::member_trigger_vote(Origin::signed(1), 1, 1));
        let expected_event = RawEvent::MemberVoteTriggered(1, 1, 1, 2);
        assert_eq!(expected_event, get_last_event());
    });
}

#[test]
fn spend_governance_works() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Bank::sudo_approve_spend_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotSudoApproveSpendProposalIfBaseBankDNE
        );
        let threshold = ThresholdInput::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Bank::summon(
            Origin::signed(1),
            1,
            20,
            Some(1),
            threshold.clone()
        ),);
        assert_noop!(
            Bank::sudo_approve_spend_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotSudoApproveSpendProposalIfSpendProposalDNE
        );
        assert_ok!(Bank::propose_spend(Origin::signed(1), 1, 10, 7),);
        assert_noop!(
            Bank::sudo_approve_spend_proposal(Origin::signed(2), 1, 1),
            Error::<Test>::NotPermittedToSudoApproveForBankAccount
        );
        assert_eq!(Balances::total_balance(&7), 77);
        assert_ok!(Bank::sudo_approve_spend_proposal(Origin::signed(1), 1, 1));
        assert_eq!(Balances::total_balance(&7), 87);
        assert_noop!(
            Bank::sudo_approve_spend_proposal(Origin::signed(1), 1, 1),
            Error::<Test>::CannotApproveAlreadyApprovedSpendProposal
        );
        assert_noop!(
            Bank::summon(Origin::signed(1), 1, 50, None, threshold),
            Error::<Test>::LimitOfOneMolochPerOrg
        );
        // register second org, same as first
        let threshold2 = ThresholdInput::new(
            OrgRep::Equal(2),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Org::register_organization(
            OrganizationSource::Accounts(vec![1, 2, 3, 4, 5, 6]),
            None,
            10
        ));
        assert_ok!(Bank::summon(Origin::signed(1), 2, 50, None, threshold2));
        assert_ok!(Bank::propose_spend(Origin::signed(3), 2, 20, 7),);
        System::set_block_number(22);
        assert_ok!(Bank::spend_trigger_vote(Origin::signed(5), 2, 1));
        for i in 1u64..7u64 {
            let i_origin = Origin::signed(i);
            assert_ok!(Vote::submit_vote(
                i_origin,
                1,
                VoterView::InFavor,
                None
            ));
        }
        assert_eq!(Balances::total_balance(&7), 87);
        run_to_block(29);
        assert_eq!(Balances::total_balance(&7), 107);
    });
}

#[test]
fn member_governance_works() {
    new_test_ext().execute_with(|| {
        let threshold = ThresholdInput::new(
            OrgRep::Equal(1),
            XorThreshold::Percent(Threshold::new(Permill::one(), None)),
        );
        assert_ok!(Bank::summon(
            Origin::signed(1),
            1,
            20,
            Some(1),
            threshold.clone()
        ),);
        assert_ok!(Bank::propose_member(Origin::signed(2), 1, 10, 1, 7),);
        assert_ok!(Bank::member_trigger_vote(Origin::signed(5), 1, 1));
        System::set_block_number(22);
        for i in 1u64..7u64 {
            let i_origin = Origin::signed(i);
            assert_ok!(Vote::submit_vote(
                i_origin,
                1,
                VoterView::InFavor,
                None
            ));
        }
        assert_eq!(Balances::total_balance(&7), 77);
        let seven_share = Org::get_share_profile(1, &7);
        assert!(seven_share.is_none());
        assert_eq!(Org::outstanding_shares(1), 6);
        run_to_block(29);
        assert_eq!(Balances::total_balance(&7), 67);
        let seven_share = Org::get_share_profile(1, &7).unwrap();
        assert_eq!(seven_share.total(), 1);
        assert_eq!(Org::outstanding_shares(1), 7);
    });
}
