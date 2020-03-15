use super::*;
use frame_support::assert_ok; // assert_err, assert_noop
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    GenesisConfig::<Test> {
        membership_shares: vec![
            (1, 1, 3),
            (2, 1, 3),
            (3, 1, 3),
            (4, 1, 3),
            (5, 1, 3),
            (1, 2, 5),
            (2, 2, 5),
            (7, 2, 5),
            (8, 2, 5),
            (9, 2, 5),
        ],
        // must equal sum of above
        total_issuance: vec![(1, 15), (2, 25)],
        // must not contradict membership_shares membership
        shareholder_membership: vec![(1, vec![1, 2, 3, 4, 5]), (2, vec![1, 2, 7, 8, 9])],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}

#[test]
fn genesis_config() {
    new_test_ext().execute_with(|| {
        for i in 1..6 {
            let free = Shares::get_free_shares(&i, 1).unwrap();
            assert_eq!(free, 3);
        }
        for j in 1..3 {
            let free = Shares::get_free_shares(&j, 2).unwrap();
            assert_eq!(free, 5);
        }
        for k in 7..10 {
            let free = Shares::get_free_shares(&k, 2).unwrap();
            assert_eq!(free, 5);
        }
        let all_one_id_shares = Shares::outstanding_shares(1);
        let all_two_id_shares = Shares::outstanding_shares(2);
        // check total issuance
        assert_eq!(all_one_id_shares, 15);
        assert_eq!(all_two_id_shares, 25);
        // TODO: check that shareholder_membership aligns with expectations
    });
}

#[test]
fn share_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis = vec![(1, 5), (2, 5)];
        assert_ok!(Shares::register_shares(one, 3, genesis));
        // get all registered shares associated with share_id == 3
        let total = Shares::outstanding_shares(3);
        // registration works as expected
        assert_eq!(total, 10);
        let one_free = Shares::get_free_shares(&1, 3).unwrap();
        let two_free = Shares::get_free_shares(&2, 3).unwrap();
        assert_eq!(one_free, 5);
        assert_eq!(two_free, 5);
    });
}

#[test]
fn share_reservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        // 5 -1
        assert_ok!(Shares::reserve_shares(one.clone(), 2, 1));
        // = 4
        let total = Shares::get_free_shares(&1, 2).unwrap();
        // check that method calculates correctly
        assert_eq!(total, 4);
        // 4 - 3
        assert_ok!(Shares::reserve_shares(one, 2, 3));
        // = 1
        let total = Shares::get_free_shares(&1, 2).unwrap();
        // check that method calculates correctly
        assert_eq!(total, 1);
    });
}

#[test]
fn share_unreservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        // 5 - 3
        assert_ok!(Shares::reserve_shares(one.clone(), 2, 3));
        // = 2
        let total = Shares::get_free_shares(&1, 2).unwrap();
        // check that method calculates correctly
        assert_eq!(total, 2);
        // 2 + 2
        assert_ok!(Shares::unreserve_shares(one, 2, 2));
        // = 4
        let total = Shares::get_free_shares(&1, 2).unwrap();
        // check that method calculates correctly
        assert_eq!(total, 4);
    });
}

#[test]
fn share_issuance() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        // issue 10 new shares to 7
        assert_ok!(Shares::issue_shares(one.clone(), 7, 1, 10));
        // check seven's share balance for share_id == 1
        let sevens_free_count = Shares::get_free_shares(&7, 1).unwrap();
        assert_eq!(sevens_free_count, 10);
        // check that all issued shares increased; before == 15 + 10 = 25
        let all_shares_w_id_one = Shares::outstanding_shares(1);
        assert_eq!(all_shares_w_id_one, 25);
    });
}

#[test]
fn share_buyback() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        // 1 reserves 2 shares
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 2));
        // 1 burns 2 reserved shares
        assert_ok!(Shares::buyback_shares(one.clone(), 1, 1, 2));
        // check seven's share balance for share_id == 1, 3 - 2 == 1
        let ones_free_count = Shares::get_free_shares(&1, 1).unwrap();
        assert_eq!(ones_free_count, 1);
        // check that all issued shares increased; before == 15 - 2 = 13
        let all_shares_w_id_one = Shares::outstanding_shares(1);
        assert_eq!(all_shares_w_id_one, 13);
    });
}
