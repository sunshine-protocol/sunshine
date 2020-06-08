use super::*;
use frame_support::{assert_noop, assert_ok};
use frame_support::{impl_outer_event, impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{testing::Header, traits::IdentityLookup, Perbill};
use util::{organization::Organization, traits::GroupMembership};

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
    // minimum deposit to register an on-chain bank
    pub const MinimumInitialDeposit: u64 = 5;
}
impl Trait for Test {
    type Event = TestEvent;
    type BankAssociatedId = u64;
    type Currency = Balances;
    type MinimumInitialDeposit = MinimumInitialDeposit;
}
pub type System = system::Module<Test>;
pub type Balances = pallet_balances::Module<Test>;
pub type Org = org::Module<Test>;
pub type Bank = Module<Test>;

fn get_last_event() -> RawEvent<u64, u64, u32, u64, u64> {
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
        let expected_organization = Organization::new(Some(1), None, constitution);
        let org_in_storage = Org::organization_states(1u64).unwrap();
        assert_eq!(expected_organization, org_in_storage);
        for i in 1u64..7u64 {
            assert!(Org::is_member_of_group(1u64, &i));
        }
        assert!(System::events().is_empty());
    });
}

// #[test]
// fn bank_registration_works() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         let one_group_controller = WithdrawalPermissions::JointOrgAccount(1);
//         // registration must be above the module minimum
//         assert_noop!(
//             Bank::register_and_seed_for_bank_account(
//                 one.clone(),
//                 4,
//                 1,
//                 one_group_controller.clone()
//             ),
//             Error::<Test>::RegistrationMustDepositAboveModuleMinimum
//         );
//         assert_ok!(Bank::register_and_seed_for_bank_account(
//             one,
//             10,
//             1,
//             weighted_share_group_controller.clone()
//         ));
//         let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
//         assert_eq!(
//             get_last_event(),
//             RawEvent::RegisteredNewOnChainBank(
//                 1,
//                 expected_treasury_id,
//                 10,
//                 1,
//                 weighted_share_group_controller
//             )
//         );
//     });
// }

// #[test]
// fn deposit_works() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         // registration test shows this treasury id is generated by registration call
//         let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
//         let null_reason = Vec::<u8>::new(); // NULL ipfs-reference
//                                             // Cannot deposit into bank account that DNE
//         assert_noop!(
//             Bank::deposit_from_signer_for_bank_account(
//                 one.clone(),
//                 expected_treasury_id,
//                 10,
//                 null_reason.clone()
//             ),
//             Error::<Test>::BankAccountNotFoundForDeposit
//         );
//         let weighted_share_group_controller =
//             WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::Weighted(1u32));
//         let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
//         assert_ok!(OrganizationWrapper::register_inner_weighted_share_group(
//             1, group
//         ));
//         assert_ok!(Bank::register_and_seed_for_bank_account(
//             one.clone(),
//             10,
//             1,
//             weighted_share_group_controller.clone()
//         ));
//         assert_ok!(Bank::deposit_from_signer_for_bank_account(
//             one,
//             expected_treasury_id,
//             10,
//             null_reason.clone()
//         ));
//         assert_eq!(
//             get_last_event(),
//             RawEvent::CapitalDepositedIntoOnChainBankAccount(
//                 1,
//                 expected_treasury_id,
//                 10,
//                 null_reason
//             )
//         );
//     });
// }

// #[test]
// fn reserve_spend_works() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         // registration test shows this treasury id is generated by registration call
//         let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
//         let null_reason = Vec::<u8>::new(); // NULL ipfs-reference
//         let weighted_share_group_controller =
//             WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::Weighted(1u32));
//         let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
//         assert_ok!(OrganizationWrapper::register_inner_weighted_share_group(
//             1, group
//         ));
//         assert_ok!(Bank::register_and_seed_for_bank_account(
//             one.clone(),
//             10,
//             1,
//             weighted_share_group_controller.clone()
//         ));
//         assert_ok!(Bank::deposit_from_signer_for_bank_account(
//             one.clone(),
//             expected_treasury_id,
//             10,
//             null_reason.clone()
//         ));
//         assert_eq!(
//             get_last_event(),
//             RawEvent::CapitalDepositedIntoOnChainBankAccount(
//                 1,
//                 expected_treasury_id,
//                 10,
//                 null_reason.clone()
//             )
//         );
//         // everything above this line was setup for reserving spends
//         assert_noop!(
//             Bank::reserve_spend_for_bank_account(
//                 one.clone(),
//                 expected_treasury_id,
//                 null_reason.clone(),
//                 21,
//                 weighted_share_group_controller.clone(),
//             ),
//             Error::<Test>::NotEnoughFundsInFreeToAllowReservation
//         );
//         // fake treasury_id
//         let fake_treasury_id = expected_treasury_id.clone().iterate();
//         assert_noop!(
//             Bank::reserve_spend_for_bank_account(
//                 one.clone(),
//                 fake_treasury_id,
//                 null_reason.clone(),
//                 21,
//                 weighted_share_group_controller.clone(),
//             ),
//             Error::<Test>::BankAccountNotFoundForSpendReservation
//         );
//         // wrong caller
//         let wrong_caller = Origin::signed(1738);
//         assert_noop!(
//             Bank::reserve_spend_for_bank_account(
//                 wrong_caller.clone(),
//                 expected_treasury_id,
//                 null_reason.clone(),
//                 10,
//                 weighted_share_group_controller.clone()
//             ),
//             Error::<Test>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
//         );
//         // self-reservation is OK see issue #71 for more info
//         assert_ok!(Bank::reserve_spend_for_bank_account(
//             one.clone(),
//             expected_treasury_id,
//             null_reason.clone(),
//             10,
//             weighted_share_group_controller.clone(),
//         ));
//         assert_eq!(
//             get_last_event(),
//             RawEvent::SpendReservedForBankAccount(
//                 1,
//                 expected_treasury_id,
//                 1,
//                 null_reason,
//                 10,
//                 weighted_share_group_controller,
//             )
//         );
//     });
// }

// #[test]
// fn commit_spend_works() {
//     new_test_ext().execute_with(|| {
//         let no_id = OnChainTreasuryID::default();
//         let one = Origin::signed(1);
//         // registration test shows this treasury id is generated by registration call
//         let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
//         let null_reason = Vec::<u8>::new(); // NULL ipfs-reference
//         let weighted_share_group_controller =
//             WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::Weighted(1u32));
//         let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
//         assert_ok!(OrganizationWrapper::register_inner_weighted_share_group(
//             1, group
//         ));
//         // no registered banks yet so we expect this error
//         assert_noop!(
//             Bank::commit_reserved_spend_for_transfer_inside_bank_account(
//                 one.clone(),
//                 no_id,
//                 1,
//                 null_reason.clone(),
//                 10,
//                 weighted_share_group_controller.clone(),
//             ),
//             Error::<Test>::BankAccountNotFoundForSpendReservation
//         );
//         // register bank account
//         assert_ok!(Bank::register_and_seed_for_bank_account(
//             one.clone(),
//             10,
//             1,
//             weighted_share_group_controller.clone()
//         ));
//         // no reserved spend yet so we can expect this error
//         assert_noop!(
//             Bank::commit_reserved_spend_for_transfer_inside_bank_account(
//                 one.clone(),
//                 expected_treasury_id,
//                 1,
//                 null_reason.clone(),
//                 10,
//                 weighted_share_group_controller.clone(),
//             ),
//             Error::<Test>::SpendReservationNotFound
//         );
//         // reserve spend
//         assert_ok!(Bank::reserve_spend_for_bank_account(
//             one.clone(),
//             expected_treasury_id,
//             null_reason.clone(),
//             10,
//             weighted_share_group_controller.clone(),
//         ));
//         // commit spend and transfer
//         assert_ok!(
//             Bank::commit_reserved_spend_for_transfer_inside_bank_account(
//                 one.clone(),
//                 expected_treasury_id,
//                 1,
//                 null_reason.clone(),
//                 10,
//                 weighted_share_group_controller.clone(),
//             )
//         );
//         assert_eq!(
//             get_last_event(),
//             RawEvent::CommitSpendBeforeInternalTransfer(
//                 1,
//                 expected_treasury_id,
//                 1,
//                 null_reason,
//                 10,
//                 weighted_share_group_controller,
//             )
//         );
//     });
// }

// #[test]
// fn unreserve_uncommitted_works() {
//     new_test_ext().execute_with(|| {
//         let one = Origin::signed(1);
//         // registration test shows this treasury id is generated by registration call
//         let expected_treasury_id = OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]);
//         let null_reason = Vec::<u8>::new(); // NULL ipfs-reference
//         let weighted_share_group_controller =
//             WithdrawalPermissions::AnyMemberOfOrgShareGroup(1u32, ShareID::Weighted(1u32));
//         let group = vec![(1, 5), (2, 5), (3, 5), (4, 5)];
//         assert_ok!(OrganizationWrapper::register_inner_weighted_share_group(
//             1, group
//         ));
//         assert_noop!(
//             Bank::unreserve_uncommitted_reservation_to_make_free(
//                 one.clone(),
//                 expected_treasury_id.clone(),
//                 1,
//                 10,
//             ),
//             Error::<Test>::BankAccountNotFoundForSpendReservation
//         );
//         // register bank account
//         assert_ok!(Bank::register_and_seed_for_bank_account(
//             one.clone(),
//             20,
//             1,
//             weighted_share_group_controller.clone()
//         ));
//         // no reserved spend yet so we can expect this error
//         assert_noop!(
//             Bank::unreserve_uncommitted_reservation_to_make_free(
//                 one.clone(),
//                 expected_treasury_id.clone(),
//                 1,
//                 10,
//             ),
//             Error::<Test>::SpendReservationNotFound
//         );
//         // reserve spend
//         assert_ok!(Bank::reserve_spend_for_bank_account(
//             one.clone(),
//             expected_treasury_id,
//             null_reason.clone(),
//             10,
//             weighted_share_group_controller.clone(),
//         ));
//         // still doesnt work if the amount exceeds the reservation amount
//         assert_noop!(
//             Bank::unreserve_uncommitted_reservation_to_make_free(
//                 one.clone(),
//                 expected_treasury_id.clone(),
//                 1,
//                 60,
//             ),
//             Error::<Test>::NotEnoughFundsInSpendReservationUnCommittedToSatisfyUnreserveUnCommittedRequest
//         );
//         let false_owner = Origin::signed(69);
//         assert_noop!(
//             Bank::unreserve_uncommitted_reservation_to_make_free(
//                 false_owner,
//                 expected_treasury_id.clone(),
//                 1,
//                 9,
//             ),
//             Error::<Test>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
//         );
//         // TODO: NotEnoughFundsInBankReservedToSatisfyUnReserveUnComittedRequest
//         assert_ok!(
//             Bank::unreserve_uncommitted_reservation_to_make_free(
//                 one,
//                 expected_treasury_id.clone(),
//                 1,
//                 9,
//             )
//         );
//     });
// }
// //TODO:
// //- unreserve committed
// //- transfer spend power
// //- spend from free by burning shares
// //- spend from reserved by referencing transfer
