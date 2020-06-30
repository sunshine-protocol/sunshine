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
    traits::IdentityLookup,
    Perbill,
};
use util::{
    organization::Organization,
    traits::GroupMembership,
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
impl org::Trait for Test {
    type Event = TestEvent;
    type IpfsReference = u32; // TODO: replace with utils_identity::Cid
    type OrgId = u64;
    type Shares = u64;
    type ReservationLimit = ReservationLimit;
}
parameter_types! {
    pub const MinimumTransfer: u64 = 10;
    pub const MinimumInitialDeposit: u64 = 20;
}
impl Trait for Test {
    type Event = TestEvent;
    type BankId = u64;
    type Currency = Balances;
    type MinimumTransfer = MinimumTransfer;
    type MinimumInitialDeposit = MinimumInitialDeposit;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Bank = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u64, u64> {
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
        balances: vec![(1, 100), (2, 98), (3, 200), (4, 75), (5, 10), (6, 69)],
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
        let expected_organization =
            Organization::new(Some(1), None, constitution);
        let org_in_storage = Org::organization_states(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

#[test]
fn opening_bank_account_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let sixnine = Origin::signed(69);
        assert_noop!(
            Bank::account_opens_account_for_org_with_deposit(
                sixnine, 1, 10, None
            ),
            Error::<Test>::CannotOpenBankAccountForOrgIfNotOrgMember
        );
        assert_noop!(
            Bank::account_opens_account_for_org_with_deposit(
                one.clone(),
                1,
                19,
                None
            ),
            Error::<Test>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum
        );
        let total_bank_count = Bank::total_bank_count();
        assert_eq!(total_bank_count, 0u32);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::AccountOpensOrgBankAccount(
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                20,
                1,
                None
            ),
        );
        let total_bank_count = Bank::total_bank_count();
        assert_eq!(total_bank_count, 1u32);
    });
}

#[test]
fn account_2_org_transfer() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        let two = Origin::signed(2);
        assert_noop!(
            Bank::account_to_org_transfer(
                two.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 0]),
                19,
            ),
            Error::<Test>::TransferFailsIfDestBankDNE
        );
        assert_noop!(
            Bank::account_to_org_transfer(
                two.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                9,
            ),
            Error::<Test>::TransferMustExceedModuleMinimum
        );
        assert_noop!(
            Bank::account_to_org_transfer(
                two.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                100,
            ),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance")
            }
        );
        assert_ok!(Bank::account_to_org_transfer(
            two.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            10,
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::AccountToOrgTransfer(
                2,
                2,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                10
            ),
        );
    });
}

#[test]
fn org_2_acc_transfer_from_transfer() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        // send 10 from 2 to org 1
        let two = Origin::signed(2);
        assert_ok!(Bank::account_to_org_transfer(
            two.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            10,
        ));
        // anything greater than 10 is not allowed
        assert_noop!(
            Bank::org_to_account_transfer_from_transfer(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                2,
                11,
            ),
            Error::<Test>::CannotTransferFromOrgToAccountIfInWrongStateOrNotEnoughFunds
        );
        // send 5 back from org 1 to 2
        assert_ok!(Bank::org_to_account_transfer_from_transfer(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            2,
            2,
            5,
        ));
        let transfer_id = TransferId::new(OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]), 2);
        // sharp state transition so spends are no longer allowed
        assert_ok!(
            Bank::stop_spends_start_withdrawals(transfer_id)
        );
        // and it is enforced
        assert_noop!(
            Bank::org_to_account_transfer_from_transfer(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                2,
                5,
            ),
            Error::<Test>::CannotTransferFromOrgToAccountIfInWrongStateOrNotEnoughFunds
        );
    });
}

#[test]
fn org_2_org_transfer_from_transfer() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        // cannot transfer to self
        assert_noop!(
            Bank::org_to_org_transfer_from_transfer(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                10
            ),
            Error::<Test>::ThisModuleDoesNotPermitTransfersToSelf
        );
        // register second bank account
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::AccountOpensOrgBankAccount(
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                1,
                20,
                1,
                None
            ),
        );
        // cannot transfer more than amount in transfer
        assert_noop!(
            Bank::org_to_org_transfer_from_transfer(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                21
            ),
            Error::<Test>::CannotTransferFromOrgToOrgIfInWrongStateOrNotEnoughFunds
        );
        // can transfer 10 because this is less than or equal to 20
        assert_ok!(Bank::org_to_org_transfer_from_transfer(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1,
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
            10
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::OrgToOrgTransferFromTransfer(TransferId { id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]), sub_id: 2 }, OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]), OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]), 10),
        );
    });
}

#[test]
fn reserve_spend_for_acc() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_noop!(
            Bank::reserve_spend_for_account(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                2,
                21
            ),
            Error::<Test>::ReserveOrgSpendExceedsFreeTransferCapital
        );
        assert_noop!(
            Bank::reserve_spend_for_account(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                2,
                5
            ),
            Error::<Test>::CannotReserveOrgSpendIfTransferDNE
        );
        assert_ok!(Bank::reserve_spend_for_account(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1,
            2,
            5
        ),);
        assert_eq!(
            get_last_event(),
            RawEvent::ReserveAccountSpendFromTransfer(
                TransferId {
                    id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    sub_id: 1
                },
                TransferId {
                    id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    sub_id: 2
                },
                2,
                5
            ),
        );
    });
}

#[test]
fn reserve_spend_for_org() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_noop!(
            Bank::reserve_spend_for_org(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                5
            ),
            Error::<Test>::TransferFailsIfDestBankDNE
        );
        // register second bank account
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_noop!(
            Bank::reserve_spend_for_org(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 3]),
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                5
            ),
            Error::<Test>::CannotReserveOrgSpendIfBankStoreDNE
        );
        assert_noop!(
            Bank::reserve_spend_for_org(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                21
            ),
            Error::<Test>::ReserveOrgSpendExceedsFreeTransferCapital
        );
        assert_noop!(
            Bank::reserve_spend_for_org(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                5
            ),
            Error::<Test>::CannotReserveOrgSpendIfTransferDNE
        );
        assert_ok!(Bank::reserve_spend_for_org(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1,
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
            5
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::ReserveOrgSpendFromTransfer(
                TransferId {
                    id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    sub_id: 1
                },
                TransferId {
                    id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                    sub_id: 2
                },
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                5
            ),
        );
    });
}

#[test]
fn transfer_reserved_spend() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_ok!(Bank::reserve_spend_for_account(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1,
            2,
            5
        ),);
        assert_noop!(
            Bank::transfer_existing_reserved_spend(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                3,
                6,
            ),
            Error::<Test>::TransferReservedFailsIfSpendReservationDNE
        );
        // odd how the first reservation is 2 but the second id is 3 so it's ok
        assert_noop!(
            Bank::transfer_existing_reserved_spend(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                6,
            ),
            Error::<Test>::TransferReservedFailsIfSpendReservationAmtIsLessThanRequest
        );
        assert_ok!(
            Bank::transfer_existing_reserved_spend(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                2,
                5,
            )
        );
        assert_eq!(
            get_last_event(),
            RawEvent::OrgToAccountReservedSpendExecuted(TransferId { id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]), sub_id: 2 }, 2, 5),
        );
        // second reserved spend
        assert_ok!(Bank::reserve_spend_for_account(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1,
            2,
            5
        ),);
        assert_ok!(
            Bank::transfer_existing_reserved_spend(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                3,
                5,
            )
        );
        assert_eq!(
            get_last_event(),
            RawEvent::OrgToAccountReservedSpendExecuted(TransferId { id: OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]), sub_id: 3 }, 2, 5),
        );
    });
}

#[test]
fn withdraw_from_org_to_acc() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Bank::account_opens_account_for_org_with_deposit(
            one.clone(),
            1,
            20,
            None
        ));
        assert_ok!(Bank::stop_transfers_to_trigger_withdrawals(
            one.clone(),
            OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
            1
        ));
        assert_noop!(
            Bank::withdraw_from_org_to_account(
                one.clone(),
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                1
            ),
            Error::<Test>::TransferNotInValidStateToMakeRequestedWithdrawal
        );
    });
}
