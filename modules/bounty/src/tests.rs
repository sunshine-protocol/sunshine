#![cfg(test)]

use super::*;
use frame_support::{assert_noop, assert_ok}; //assert_err
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};

// type aliases
pub type AccountId = u64;
pub type Shares = u64;
pub type Signal = u64;
pub type BlockNumber = u64;

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

mod bounty {
    pub use super::super::*;
}

impl_outer_event! {
    pub enum TestEvent for TestRuntime {
        system<T>,
        pallet_balances<T>,
        membership<T>,
        shares_atomic<T>,
        shares_membership<T>,
        org<T>,
        bank_onchain<T>,
        vote_yesno<T>,
        vote_petition<T>,
        bounty<T>,
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
parameter_types! {
    pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Trait for TestRuntime {
    type Balance = u64;
    type Event = TestEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
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
impl org::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = OrgMembership;
    type FlatShareData = FlatShareData;
    type WeightedShareData = WeightedShareData;
}
parameter_types! {
    pub const MinimumInitialDeposit: u64 = 5;
}
impl bank_onchain::Trait for TestRuntime {
    type Event = TestEvent;
    type Currency = Balances;
    type Organization = OrganizationInterface;
    type MinimumInitialDeposit = MinimumInitialDeposit;
}
impl vote_petition::Trait for TestRuntime {
    type Event = TestEvent;
    type OrgData = membership::Module<TestRuntime>;
    type ShareData = shares_membership::Module<TestRuntime>;
}
impl vote_yesno::Trait for TestRuntime {
    type Event = TestEvent;
    type Signal = Signal;
    type OrgData = membership::Module<TestRuntime>;
    type FlatShareData = shares_membership::Module<TestRuntime>;
    type WeightedShareData = shares_atomic::Module<TestRuntime>;
}
parameter_types! {
    pub const MinimumBountyCollateralRatio: Permill = Permill::from_percent(20);
    pub const BountyLowerBound: u64 = 10;
}
impl Trait for TestRuntime {
    type Event = TestEvent;
    type Currency = Balances;
    type Organization = OrganizationInterface;
    type VotePetition = VotePetition;
    type VoteYesNo = VoteYesNo;
    type Bank = Bank;
    type MinimumBountyCollateralRatio = MinimumBountyCollateralRatio;
    type BountyLowerBound = BountyLowerBound;
}
pub type System = system::Module<TestRuntime>;
pub type Balances = pallet_balances::Module<TestRuntime>;
pub type OrgMembership = membership::Module<TestRuntime>;
pub type FlatShareData = shares_membership::Module<TestRuntime>;
pub type WeightedShareData = shares_atomic::Module<TestRuntime>;
pub type OrganizationInterface = org::Module<TestRuntime>;
pub type Bank = bank_onchain::Module<TestRuntime>;
pub type VoteYesNo = vote_yesno::Module<TestRuntime>;
pub type VotePetition = vote_petition::Module<TestRuntime>;
pub type Bounty = Module<TestRuntime>;

fn get_last_event() -> RawEvent<u64, u64, ApplicationState<u64>> {
    System::events()
        .into_iter()
        .map(|r| r.event)
        .filter_map(|e| {
            if let TestEvent::bounty(inner) = e {
                Some(inner)
            } else {
                None
            }
        })
        .last()
        .unwrap()
}

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();
    pallet_balances::GenesisConfig::<TestRuntime> {
        balances: vec![(1, 100), (2, 98), (3, 200), (4, 75), (5, 10), (6, 69)],
    }
    .assimilate_storage(&mut t)
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
    org::GenesisConfig::<TestRuntime> {
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

use util::{organization::Organization, traits::GroupMembership};

#[test]
fn genesis_config_works() {
    new_test_ext().execute_with(|| {
        assert_eq!(OrganizationInterface::organization_counter(), 1);
        let constitution = b"build cool shit".to_vec();
        let expected_organization = Organization::new(ShareID::Flat(1u32), constitution.clone());
        let org_in_storage = OrganizationInterface::organization_states(1u32).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        // check membership from membership module
        for i in 1u64..7u64 {
            assert!(OrgMembership::is_member_of_group(1u32, &i));
        }
        assert!(System::events().is_empty());
    });
}

#[test]
fn register_foundation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
    });
}

#[test]
fn post_bounty() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        // -> I would put this in a separate test but fuck all that boilerplate repeated in every test
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
    });
}

#[test]
fn submit_grant_app() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        // -> I would put this in a separate test but fuck all that boilerplate repeated in every test
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
    });
}

#[test]
fn trigger_app_review() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        // -> I would put this in a separate test but fuck all that boilerplate repeated in every test
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- TRIGGER APPLICATION REVIEW
        assert_noop!(
            Bounty::direct__trigger_application_review(one.clone(), 2, 1,),
            Error::<TestRuntime>::CannotReviewApplicationIfBountyDNE
        );
        assert_noop!(
            Bounty::direct__trigger_application_review(one.clone(), 1, 2,),
            Error::<TestRuntime>::CannotReviewApplicationIfApplicationDNE
        );
        // caller from outside of the acceptance committee
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bounty::direct__trigger_application_review(sixnine.clone(), 1, 1,),
            Error::<TestRuntime>::AccountNotAuthorizedToTriggerApplicationReview
        );
        assert_ok!(Bounty::direct__trigger_application_review(
            one.clone(),
            1,
            1,
        ));
    });
}

#[test]
fn sudo_approve_grant_app() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- SUDO APPROVE GRANT APPLICATION
        assert_noop!(
            Bounty::direct__sudo_approve_application(one.clone(), 2, 1,),
            Error::<TestRuntime>::CannotSudoApproveIfBountyDNE
        );
        assert_noop!(
            Bounty::direct__sudo_approve_application(one.clone(), 1, 2,),
            Error::<TestRuntime>::CannotSudoApproveIfGrantAppDNE
        );
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bounty::direct__sudo_approve_application(sixnine.clone(), 1, 1,),
            Error::<TestRuntime>::CannotSudoApproveAppIfNotAssignedSudo
        );
        assert_ok!(Bounty::direct__sudo_approve_application(one.clone(), 1, 1,));
        assert_noop!(
            Bounty::direct__sudo_approve_application(one.clone(), 1, 1,),
            Error::<TestRuntime>::AppStateCannotBeSudoApprovedForAGrantFromCurrentState
        );
    });
}

#[test]
fn poll_application_status_from_review_board_path_to_team_consent() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- POLL APPLICATION
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::SubmittedAwaitingResponse,
            )
        );
        assert_ok!(Bounty::direct__trigger_application_review(
            one.clone(),
            1,
            1,
        ));
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::UnderReviewByAcceptanceCommittee(VoteID::Petition(1)),
            )
        );
        // ShareID::Flat(1) = {1, 2, 3, 4}
        // add 3 approvals to meet threshold for `new_review_board` declared and assigned further above
        for i in 1u64..4u64 {
            assert_ok!(VotePetition::sign_petition(
                1,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                    ShareID::Flat(2),
                    VoteID::Petition(2)
                ),
            )
        );
    });
} // team consent -> approval in next test

#[test]
fn poll_application_status_from_sudo_approve_to_team_consent_to_approval() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- POLL APPLICATION
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::SubmittedAwaitingResponse,
            )
        );
        assert_ok!(Bounty::direct__sudo_approve_application(one.clone(), 1, 1,));
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                    ShareID::Flat(2),
                    VoteID::Petition(1)
                ),
            )
        );
        // ShareID::Flat(2) = {1, 2, 3, 4}
        // impl UNANIMOUS CONSENT
        for i in 1u64..5u64 {
            assert_ok!(VotePetition::sign_petition(
                1,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        let expected_team_id = TeamID::new(1, Some(1), 2, 2);
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedAndLive(expected_team_id),
            )
        );
    });
}

#[test]
fn submit_milestone() {
    new_test_ext().execute_with(|| {
        // !!!!!BOILERPLATE STARTS!!!!!
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            5,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                5,                        // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- POLL APPLICATION
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::SubmittedAwaitingResponse,
            )
        );
        assert_ok!(Bounty::direct__sudo_approve_application(one.clone(), 1, 1,));
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                    ShareID::Flat(2),
                    VoteID::Petition(1)
                ),
            )
        );
        // ShareID::Flat(2) = {1, 2, 3, 4}
        // impl UNANIMOUS CONSENT
        for i in 1u64..5u64 {
            assert_ok!(VotePetition::sign_petition(
                1,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        let expected_team_id = TeamID::new(1, Some(1), 2, 2);
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedAndLive(expected_team_id),
            )
        );
        // !!!!!BOILERPLATE ENDS!!!!!
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                11,
            ),
            Error::<TestRuntime>::MilestoneSubmissionRequestExceedsApprovedApplicationsLimit
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                2,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                2,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        let fake_team_id = TeamID::new(2, Some(1), 2, 2);
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                fake_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::ApplicationMustApprovedAndLiveWithTeamIDMatchingInput
        );
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bounty::direct__submit_milestone(
                sixnine.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CallerMustBeMemberOfFlatShareGroupToSubmitMilestones
        );
        assert_ok!(Bounty::direct__submit_milestone(
            one.clone(),
            1,
            1,
            expected_team_id,
            IpfsReference::default(),
            10,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneSubmitted(
                1, // submitter id
                1, // bounty id
                1, // grant app id
                1, // milestone id
            )
        );
    });
}

#[test]
fn trigger_milestone_review_to_review_board_approval_and_transfer() {
    new_test_ext().execute_with(|| {
        // !!!BOILERPLATE STARTS!!!
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            10,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                10,                       // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- POLL APPLICATION
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::SubmittedAwaitingResponse,
            )
        );
        assert_ok!(Bounty::direct__sudo_approve_application(one.clone(), 1, 1,));
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                    ShareID::Flat(2),
                    VoteID::Petition(1)
                ),
            )
        );
        // ShareID::Flat(2) = {1, 2, 3, 4}
        // impl UNANIMOUS CONSENT
        for i in 1u64..5u64 {
            assert_ok!(VotePetition::sign_petition(
                1,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        let expected_team_id = TeamID::new(1, Some(1), 2, 2);
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedAndLive(expected_team_id),
            )
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                11,
            ),
            Error::<TestRuntime>::MilestoneSubmissionRequestExceedsApprovedApplicationsLimit
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                2,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                2,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        let fake_team_id = TeamID::new(2, Some(1), 2, 2);
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                fake_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::ApplicationMustApprovedAndLiveWithTeamIDMatchingInput
        );
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bounty::direct__submit_milestone(
                sixnine.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CallerMustBeMemberOfFlatShareGroupToSubmitMilestones
        );
        assert_ok!(Bounty::direct__submit_milestone(
            one.clone(),
            1,
            1,
            expected_team_id,
            IpfsReference::default(),
            10,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneSubmitted(
                1, // submitter id
                1, // bounty id
                1, // grant app id
                1, // milestone id
            )
        );
        // !!!BOILERPLATE ENDS!!!

        // trigger milestone review
        assert_noop!(
            Bounty::direct__trigger_milestone_review(one.clone(), 2, 1,),
            Error::<TestRuntime>::CannotTriggerMilestoneReviewIfBountyDNE
        );
        assert_noop!(
            Bounty::direct__trigger_milestone_review(one.clone(), 1, 2,),
            Error::<TestRuntime>::CannotTriggerMilestoneReviewIfMilestoneSubmissionDNE
        );
        assert_ok!(Bounty::direct__trigger_milestone_review(one.clone(), 1, 1,));
        assert_noop!(
            Bounty::direct__trigger_milestone_review(one.clone(), 1, 1,),
            Error::<TestRuntime>::SubmissionIsNotReadyForReview
        );
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneReviewTriggered(
                1, // trigger account_id
                1, // bounty id
                1, // milestone id
                MilestoneStatus::SubmittedReviewStarted(VoteID::Petition(2)),
            )
        );
        // vote the milestone review through to passage
        // ShareID::Flat(1) = {1, 2, 3, 4}
        // 3 approvals to meet threshold
        for i in 1u64..4u64 {
            assert_ok!(VotePetition::sign_petition(
                2,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        // poll the milestone
        assert_ok!(Bounty::direct__poll_milestone(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestonePolled(
                1, // poller account_id
                1, // bounty id
                1, // milestone id
                MilestoneStatus::ApprovedAndTransferEnabled(
                    OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    1
                )
            )
        );
    });
}

#[test]
fn sudo_approve_milestone_and_transfer() {
    new_test_ext().execute_with(|| {
        // !!!BOILERPLATE STARTS!!!
        let one = Origin::signed(1);
        let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
        // bank-onchain registration boilerplate
        let weighted_share_group_controller =
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::WeightedAtomic(1u32));
        let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
        assert_ok!(OrganizationInterface::register_inner_weighted_share_group(
            1, group
        ));
        assert_ok!(Bank::register_on_chain_bank_account(
            1,
            1,
            10,
            weighted_share_group_controller.clone()
        ));
        // -- TEST 1 -- REGISTER FOUNDATION FROM EXISTING ONCHAIN BANK ACCOUNT
        // 1 is not the bank owner for this fake_treasury_id
        let fake_treasury_id = OnChainTreasuryID::default();
        assert_noop!(
            Bounty::direct__register_foundation_from_existing_bank(
                one.clone(),
                1,
                fake_treasury_id
            ),
            Error::<TestRuntime>::CannotRegisterFoundationFromOrgBankRelationshipThatDNE
        );
        assert_ok!(Bounty::direct__register_foundation_from_existing_bank(
            one.clone(),
            1,
            expected_treasury_id
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationRegisteredFromOnChainBank(1, expected_treasury_id,)
        );
        // -- TEST 2 -- Create Bounty
        let new_review_board = ReviewBoard::new_flat_petition_review(Some(1), 1, 1, 3, None, None);
        // Cannot open a bounty below the minimum for this module
        assert_noop!(
            Bounty::direct__create_bounty(
                one.clone(),
                1,
                IpfsReference::default(),
                expected_treasury_id,
                1,
                2,
                new_review_board.clone(),
                None,
            ),
            Error::<TestRuntime>::MinimumBountyClaimedAmountMustMeetModuleLowerBound
        );
        assert_ok!(Bounty::direct__create_bounty(
            one.clone(),
            1,
            IpfsReference::default(),
            expected_treasury_id,
            10,
            10,
            new_review_board,
            None,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FoundationPostedBounty(
                1, // bounty_creator
                1, // registered org id
                1, // bounty identifier
                expected_treasury_id,
                IpfsReference::default(), // description
                10,                       // amount reserved for bounty
                10                        // amount claimed available for bounty
            )
        );
        // -- TEST 3 -- SUBMIT GRANT APPLICATION
        let team_one_share_metadata = vec![(1, 10), (2, 10), (3, 10), (4, 10)];
        let team_one_terms_of_agreement = TermsOfAgreement::new(Some(1), team_one_share_metadata);
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                1,
                IpfsReference::default(),
                11,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantRequestExceedsAvailableBountyFunds
        );
        assert_noop!(
            Bounty::direct__submit_grant_application(
                one.clone(),
                2,
                IpfsReference::default(),
                10,
                team_one_terms_of_agreement.clone()
            ),
            Error::<TestRuntime>::GrantApplicationFailsIfBountyDNE
        );
        assert_ok!(Bounty::direct__submit_grant_application(
            one.clone(),
            1,
            IpfsReference::default(),
            10,
            team_one_terms_of_agreement
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::GrantApplicationSubmittedForBounty(
                1,                        // submitter
                1,                        // bounty id
                1,                        // grant app id
                IpfsReference::default(), // description
                10,
            )
        );
        // -- TEST 4 -- POLL APPLICATION
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::SubmittedAwaitingResponse,
            )
        );
        assert_ok!(Bounty::direct__sudo_approve_application(one.clone(), 1, 1,));
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedByFoundationAwaitingTeamConsent(
                    ShareID::Flat(2),
                    VoteID::Petition(1)
                ),
            )
        );
        // ShareID::Flat(2) = {1, 2, 3, 4}
        // impl UNANIMOUS CONSENT
        for i in 1u64..5u64 {
            assert_ok!(VotePetition::sign_petition(
                1,
                i,
                util::petition::PetitionView::Assent(IpfsReference::default()),
            ));
        }
        assert_ok!(Bounty::any_acc__poll_application(one.clone(), 1, 1,));
        let expected_team_id = TeamID::new(1, Some(1), 2, 2);
        assert_eq!(
            get_last_event(),
            RawEvent::ApplicationPolled(
                1, // bounty id
                1, // grant app id
                ApplicationState::ApprovedAndLive(expected_team_id),
            )
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                11,
            ),
            Error::<TestRuntime>::MilestoneSubmissionRequestExceedsApprovedApplicationsLimit
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                2,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                2,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CannotSubmitMilestoneIfApplicationDNE
        );
        let fake_team_id = TeamID::new(2, Some(1), 2, 2);
        assert_noop!(
            Bounty::direct__submit_milestone(
                one.clone(),
                1,
                1,
                fake_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::ApplicationMustApprovedAndLiveWithTeamIDMatchingInput
        );
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bounty::direct__submit_milestone(
                sixnine.clone(),
                1,
                1,
                expected_team_id,
                IpfsReference::default(),
                10,
            ),
            Error::<TestRuntime>::CallerMustBeMemberOfFlatShareGroupToSubmitMilestones
        );
        assert_ok!(Bounty::direct__submit_milestone(
            one.clone(),
            1,
            1,
            expected_team_id,
            IpfsReference::default(),
            10,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestoneSubmitted(
                1, // submitter id
                1, // bounty id
                1, // grant app id
                1, // milestone id
            )
        );
        // !!!BOILERPLATE ENDS!!!

        // sudo approve
        assert_noop!(
            Bounty::direct__sudo_approves_milestone(sixnine.clone(), 1, 1),
            Error::<TestRuntime>::CannotSudoApproveMilestoneIfNotAssignedSudo
        );
        assert_ok!(Bounty::direct__sudo_approves_milestone(one.clone(), 1, 1));
        assert_noop!(
            Bounty::direct__sudo_approves_milestone(one.clone(), 1, 1),
            Error::<TestRuntime>::SubmissionIsNotReadyForReview
        );
        // poll the milestone
        assert_ok!(Bounty::direct__poll_milestone(one.clone(), 1, 1,));
        assert_eq!(
            get_last_event(),
            RawEvent::MilestonePolled(
                1, // poller account_id
                1, // bounty id
                1, // milestone id
                MilestoneStatus::ApprovedAndTransferEnabled(
                    OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    1
                )
            )
        );
    });
}
