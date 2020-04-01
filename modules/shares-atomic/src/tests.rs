use super::*;
use frame_support::assert_ok; // assert_err, assert_noop
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    GenesisConfig::<Test> {
        membership_shares: vec![
            // org, share_id, account, amount: shares
            // organization 1
            (1, 1, 1, 10),
            (1, 1, 2, 10),
            (1, 1, 3, 10),
            (1, 1, 4, 10),
            (1, 1, 5, 10),
            (1, 1, 6, 10),
            (1, 1, 7, 10),
            (1, 1, 8, 10),
            (1, 1, 9, 10),
            (1, 1, 10, 10),
            (1, 2, 1, 10),
            (1, 2, 2, 10),
            (1, 2, 3, 10),
            (1, 2, 4, 10),
            (1, 2, 5, 10),
            (1, 3, 6, 20),
            (1, 3, 7, 20),
            (1, 3, 8, 20),
            (1, 3, 9, 20),
            (1, 3, 10, 20),
            // organization 2
            (2, 1, 11, 10),
            (2, 1, 12, 10),
            (2, 1, 13, 10),
            (2, 1, 14, 10),
            (2, 1, 15, 10),
            (2, 1, 16, 10),
            (2, 1, 17, 10),
            (2, 1, 18, 10),
            (2, 1, 19, 10),
        ],
        // must equal sum of above
        total_issuance: vec![(1, 1, 100), (1, 2, 50), (1, 3, 100), (2, 1, 90)],
        // must not contradict membership_shares membership
        shareholder_membership: vec![
            (1, 1, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            (1, 2, vec![1, 2, 3, 4, 5]),
            (1, 3, vec![6, 7, 8, 9, 10]),
            (2, 1, vec![11, 12, 13, 14, 15, 16, 17, 18, 19]),
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}

#[test]
fn genesis_config() {
    new_test_ext().execute_with(|| {
        let one_one_id_shares = Shares::outstanding_shares(1, 1);
        let one_two_id_shares = Shares::outstanding_shares(1, 2);
        let one_three_id_shares = Shares::outstanding_shares(1, 3);
        let two_one_id_shares = Shares::outstanding_shares(2, 1);
        // check total issuance
        assert_eq!(one_one_id_shares, 100);
        assert_eq!(one_two_id_shares, 50);
        assert_eq!(one_three_id_shares, 100);
        assert_eq!(two_one_id_shares, 90);
        // TODO: check that shareholder_membership aligns with expectations
    });
}

#[test]
fn check_membership() {
    // constant time membership lookups
    new_test_ext().execute_with(|| {
        let mut n = 0u64;
        let first_group_id = (1, 1);
        let second_group_id = (1, 2);
        let third_group_id = (1, 3);
        // different organization
        let second_first_group_id = (2, 1);
        while n < 19 {
            n += 1;
            if n < 11 {
                if n < 6 {
                    assert!(Shares::is_member_of_group(second_group_id, &n));
                } else {
                    assert!(Shares::is_member_of_group(third_group_id, &n));
                }
                assert!(Shares::is_member_of_group(first_group_id, &n));
            } else {
                assert!(Shares::is_member_of_group(second_first_group_id, &n));
            }
        }
    });
}

#[test]
fn share_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis = vec![
            // account_id, shares
            (1, 5),
            (2, 5),
        ];
        assert_ok!(Shares::register_shares(one, 1, 1, genesis));
        // get all registered shares associated with share_id == 3
        let total = Shares::outstanding_shares(1, 1);
        // registration works as expected
        assert_eq!(total, 100);
    });
}

#[test]
fn share_reservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 1));
        let prefix_key = (1, 1);
        let profile = Shares::profile(prefix_key, 1).unwrap();
        let first_times_reserved = profile.get_times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_reserved, 1);
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 1));
        let second_profile = Shares::profile(prefix_key, 1).unwrap();
        let second_times_reserved = second_profile.get_times_reserved();
        assert_eq!(second_times_reserved, 2);
        let mut n = 0u32;
        while n < 20 {
            assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 1));
            n += 1;
        }
        let n_profile = Shares::profile(prefix_key, 1).unwrap();
        let n_times_reserved = n_profile.get_times_reserved();
        assert_eq!(n_times_reserved, 22);

        // check same logic with another member of the first group
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 2));
        let a_prefix_key = (1, 1);
        let a_profile = Shares::profile(a_prefix_key, 2).unwrap();
        let a_first_times_reserved = a_profile.get_times_reserved();
        // // check that method calculates correctly
        assert_eq!(a_first_times_reserved, 1);
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 2));
        let a_second_profile = Shares::profile(a_prefix_key, 2).unwrap();
        let a_second_times_reserved = a_second_profile.get_times_reserved();
        assert_eq!(a_second_times_reserved, 2);
        let mut a_n = 0u32;
        while a_n < 20 {
            assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 2));
            a_n += 1;
        }
        let a_n_profile = Shares::profile(a_prefix_key, 2).unwrap();
        let a_n_times_reserved = a_n_profile.get_times_reserved();
        assert_eq!(a_n_times_reserved, 22);
    });
}

#[test]
fn share_unreservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(Shares::reserve_shares(one.clone(), 1, 1, 1));
        let prefix_key = (1, 1);
        let profile = Shares::profile(prefix_key, 1).unwrap();
        let first_times_reserved = profile.get_times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_reserved, 1);
        assert_ok!(Shares::unreserve_shares(one.clone(), 1, 1, 1));
        let un_profile = Shares::profile(prefix_key, 1).unwrap();
        let first_times_un_reserved = un_profile.get_times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_un_reserved, 0);
    });
}

#[test]
fn share_lock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = (1, 1);
        let profile = Shares::profile(prefix_key, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(Shares::lock_shares(one.clone(), 1, 1, 1));
        let locked_profile = Shares::profile(prefix_key, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
    });
}

#[test]
fn share_unlock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = (1, 1);
        let profile = Shares::profile(prefix_key, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(Shares::lock_shares(one.clone(), 1, 1, 1));
        let locked_profile = Shares::profile(prefix_key, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
        assert_ok!(Shares::unlock_shares(one.clone(), 1, 1, 1));
        let unlocked_profile = Shares::profile(prefix_key, 1).unwrap();
        let is_unlocked = unlocked_profile.is_unlocked();
        assert_eq!(is_unlocked, true);
    });
}

#[test]
fn share_issuance() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = (1, 1);
        let pre_profile = Shares::profile(prefix_key, 10).unwrap();
        let pre_shares = pre_profile.get_shares();

        assert_eq!(pre_shares, 10);
        // issue 10 new shares to 7
        assert_ok!(Shares::issue_shares(one.clone(), 1, 1, 10, 10));

        let post_profile = Shares::profile(prefix_key, 10).unwrap();
        let post_shares = post_profile.get_shares();

        assert_eq!(post_shares, 20);
    });
}

#[test]
fn share_burn() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = (1, 1);
        let pre_profile = Shares::profile(prefix_key, 10).unwrap();
        let pre_shares = pre_profile.get_shares();

        assert_eq!(pre_shares, 10);
        // issue 10 new shares to 10
        assert_ok!(Shares::issue_shares(one.clone(), 1, 1, 10, 10));

        let pre_pre_profile = Shares::profile(prefix_key, 10).unwrap();
        let pre_pre_shares = pre_pre_profile.get_shares();

        assert_eq!(pre_pre_shares, 20);
        // burn 10 new shares for 10
        assert_ok!(Shares::burn_shares(one.clone(), 1, 1, 10, 10));
        let post_profile = Shares::profile(prefix_key, 10).unwrap();
        let post_shares = post_profile.get_shares();

        assert_eq!(post_shares, 10);
    });
}
