use super::*;
use frame_support::{assert_noop, assert_ok};
use mock::*;

fn get_last_event() -> RawEvent<u64, u64> {
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
    GenesisConfig::<Test> {
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
        assert_eq!(Bank::organization_counter(), 1);
        let constitution = b"build cool shit".to_vec();
        let expected_organization = Organization::new(ShareID::Flat(1u32), constitution.clone());
        let org_in_storage = Bank::organization_states(1u32).unwrap();
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
        assert_ok!(Bank::register_organization_from_accounts(
            one.clone(),
            constitution.clone().to_vec(),
            accounts,
            Some(1),
        ));
        // check organization count changed as expected
        assert_eq!(Bank::organization_counter(), 2);
        // Event Emittance in Tests Consistently Fails -- this mystery needs to be solved in order to test...
        assert_eq!(
            get_last_event(),
            RawEvent::NewOrganizationRegistered(1, 2, ShareID::Flat(1), constitution.to_vec()),
        );
        let third_org_accounts = vec![1, 2, 3, 9, 10];
        let third_org_constitution: &[u8] = b"no talking about fight club";
        // next line is registration call
        assert_ok!(Bank::register_organization_from_accounts(
            one.clone(),
            third_org_constitution.clone().to_vec(),
            third_org_accounts,
            Some(1),
        ));
        // check organization count changed as expected
        assert_eq!(Bank::organization_counter(), 3);
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
        assert_ok!(Bank::register_inner_flat_share_group_for_organization(
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
        assert_ok!(Bank::register_inner_flat_share_group_for_organization(
            one.clone(),
            1u32,
            second_share_group
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::FlatInnerShareGroupAddedToOrg(1, 1, ShareID::Flat(3)),
        );
        let third_share_group = vec![1, 2, 3, 9, 10, 11, 12, 13, 14, 17, 18, 19, 30];
        assert_ok!(Bank::register_inner_flat_share_group_for_organization(
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
            assert!(Bank::organization_inner_shares(1, ShareID::Flat(i)));
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
        assert_ok!(Bank::register_inner_weighted_share_group_for_organization(
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
        assert_ok!(Bank::register_inner_weighted_share_group_for_organization(
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
        assert_ok!(Bank::register_inner_weighted_share_group_for_organization(
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
        assert_ok!(Bank::register_inner_weighted_share_group_for_organization(
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

#[test]
fn offchain_bank_functionality() {
    new_test_ext().execute_with(|| {
        // check if it works for the first group
        let one = Origin::signed(1);
        assert_ok!(Bank::register_offchain_bank_account_for_organization(
            one.clone(),
            1u32
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::NewOffChainTreasuryRegisteredForOrg(1, 1),
        );
        // an account in the org uses it to log a payment
        let six = Origin::signed(6);
        let sixtynine = Origin::signed(69);
        assert_noop!(
            Bank::use_offchain_bank_account_to_claim_payment_sent(sixtynine.clone(), 1, 69, 69),
            Error::<Test>::MustBeAMemberToUseOffChainBankAccountToClaimPaymentSent
        );
        assert_ok!(Bank::use_offchain_bank_account_to_claim_payment_sent(
            six, 1, 69, 69
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::SenderClaimsPaymentSent(1, 6, 69, 69, 0),
        );
        // note how this error is returned because one is NOT the recipient
        assert_noop!(
            Bank::use_offchain_bank_account_to_confirm_payment_received(one, 1, 0, 6, 69),
            Error::<Test>::SenderMustClaimPaymentSentForRecipientToConfirm
        );
        assert_ok!(Bank::use_offchain_bank_account_to_confirm_payment_received(
            sixtynine, 1, 0, 6, 69
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::RecipientConfirmsPaymentReceived(1, 6, 69, 69, 0),
        );
    });
}

#[test]
fn on_chain_bank_functionality_sudo_permissions() {
    new_test_ext().execute_with(|| {
        // traditional bank account ACL
        let one = Origin::signed(1);
        let sixtynine = Origin::signed(69);
        assert_noop!(
            Bank::register_on_chain_bank_account_with_sudo_permissions(sixtynine, 20, 2),
            Error::<Test>::MustHaveCertainAuthorityToRegisterOnChainBankAccount
        );
        assert_noop!(
            Bank::register_on_chain_bank_account_with_sudo_permissions(one.clone(), 120, 2),
            DispatchError::Module {
                index: 0,
                error: 3,
                message: Some("InsufficientBalance",),
            }
        );
        assert_ok!(Bank::register_on_chain_bank_account_with_sudo_permissions(
            one.clone(),
            20,
            2
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::NewOnChainTreasuryRegisteredWithSudoPermissions(
                OnChainTreasuryID([0u8; 8]),
                2
            ),
        );
        assert_ok!(Bank::register_on_chain_bank_account_with_sudo_permissions(
            one.clone(),
            20,
            3
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::NewOnChainTreasuryRegisteredWithSudoPermissions(
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 1]),
                3
            ),
        );
        assert_ok!(Bank::register_on_chain_bank_account_with_sudo_permissions(
            one.clone(),
            20,
            4
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::NewOnChainTreasuryRegisteredWithSudoPermissions(
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 2]),
                4
            ),
        );
        // sudo withdrawals
        let two = Origin::signed(2);
        let three = Origin::signed(3);
        assert_noop!(
            Bank::sudo_withdrawal_from_on_chain_bank_account(
                two.clone(),
                OnChainTreasuryID([1u8; 8]),
                2,
                20
            ),
            Error::<Test>::CannotWithdrawIfOnChainBankDNE
        );
        assert_noop!(
            Bank::sudo_withdrawal_from_on_chain_bank_account(
                three.clone(),
                OnChainTreasuryID([0u8; 8]),
                2,
                20
            ),
            Error::<Test>::BankAccountEitherNotSudoOrCallerIsNotDesignatedSudo
        );
        assert_noop!(
            Bank::sudo_withdrawal_from_on_chain_bank_account(
                two.clone(),
                OnChainTreasuryID([0u8; 8]),
                2,
                21
            ),
            Error::<Test>::WithdrawalRequestExceedsFundsAvailableForSpend
        );
        assert_ok!(Bank::sudo_withdrawal_from_on_chain_bank_account(
            two.clone(),
            OnChainTreasuryID([0u8; 8]),
            2,
            20
        ));
        assert_eq!(
            get_last_event(),
            RawEvent::SudoWithdrawalFromOnChainBankAccount(
                OnChainTreasuryID([0, 0, 0, 0, 0, 0, 0, 0]),
                2,
                20
            ),
        );
        // check the bank's balance
        let bank_id = OnChainTreasuryID([0u8; 8]);
        let bank_account = Bank::on_chain_treasury_ids(bank_id).unwrap();
        assert_eq!(bank_account.savings(), 0);
        assert_eq!(bank_account.reserved_for_spends(), 0);
        // TODO: consider what state gc policy for bank accounts with 0
    });
}
