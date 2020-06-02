use super::*;
use frame_support::assert_ok; // assert_noop
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    membership::GenesisConfig::<Test> {
        omnipotent_key: 1,
        membership: Some(vec![
            // org, account
            (1, 1, true),
            (1, 2, true),
            (1, 3, true),
            (1, 4, true),
            (1, 5, true),
            (1, 6, true),
            (1, 7, true),
            (1, 8, true),
            (1, 9, true),
            (1, 10, true),
            (1, 11, true),
            (1, 12, true),
            (2, 8, true),
            (2, 9, true),
            (2, 10, true),
            (2, 11, true),
            (2, 12, true),
            (3, 1, true),
            (3, 2, true),
            (3, 3, true),
            (3, 4, true),
            (4, 1, true),
            (4, 2, true),
            (4, 10, true),
            (4, 11, true),
            (4, 12, true),
            (5, 1, true),
            (5, 2, true),
            (5, 3, true),
            (5, 4, true),
            (5, 5, true),
            (5, 6, true),
            (5, 7, true),
            (5, 8, true),
            (5, 9, true),
            (5, 10, true),
            (5, 11, true),
            (5, 12, true),
            (5, 13, true),
            (5, 14, true),
            (5, 15, true),
            (5, 16, true),
            (5, 17, true),
            (5, 18, true),
            (5, 19, true),
            (5, 20, true),
        ]),
    }
    .assimilate_storage(&mut t)
    .unwrap();
    GenesisConfig::<Test> {
        share_supervisors: Some(vec![(1, 1, 1), (1, 2, 10), (1, 3, 3), (2, 1, 1)]),
        shareholder_membership: Some(vec![
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
            (1, 2, 8, 10),
            (1, 2, 9, 10),
            (1, 2, 10, 10),
            (1, 2, 11, 10),
            (1, 2, 12, 10),
            (1, 3, 1, 20),
            (1, 3, 2, 20),
            (1, 3, 3, 20),
            (1, 3, 4, 20),
            // organization 2
            (2, 1, 1, 10),
            (2, 1, 12, 10),
            (2, 1, 13, 10),
            (2, 1, 14, 10),
            (2, 1, 15, 10),
            (2, 1, 16, 10),
            (2, 1, 17, 10),
            (2, 1, 18, 10),
            (2, 1, 19, 10),
        ]),
    }
    .assimilate_storage(&mut t)
    .unwrap();
    t.into()
}

#[test]
fn genesis_config() {
    new_test_ext().execute_with(|| {
        let one_one_id_shares = AtomicShares::outstanding_shares(1, 1).unwrap();
        let one_two_id_shares = AtomicShares::outstanding_shares(1, 2).unwrap();
        let one_three_id_shares = AtomicShares::outstanding_shares(1, 3).unwrap();
        let two_one_id_shares = AtomicShares::outstanding_shares(2, 1).unwrap();
        // check total issuance
        assert_eq!(one_one_id_shares, 100);
        assert_eq!(one_two_id_shares, 50);
        assert_eq!(one_three_id_shares, 80);
        assert_eq!(two_one_id_shares, 90);
        // TODO: check that shareholder_membership aligns with expectations
    });
}

#[test]
fn check_membership() {
    // constant time membership lookups
    new_test_ext().execute_with(|| {
        let mut n = 0u64;
        let first_group_id = ShareGroup::new(1, 1);
        let second_group_id = ShareGroup::new(1, 2);
        let third_group_id = ShareGroup::new(1, 3);
        // different organization
        let second_first_group_id = ShareGroup::new(2, 1);
        while n < 19 {
            n += 1;
            if n == 1 {
                assert!(AtomicShares::is_member_of_group(second_first_group_id, &n));
            }
            if n < 11 {
                if n < 5 {
                    assert!(AtomicShares::is_member_of_group(third_group_id, &n));
                } else if n > 8 {
                    assert!(AtomicShares::is_member_of_group(second_group_id, &n));
                }
                assert!(AtomicShares::is_member_of_group(first_group_id, &n));
            } else {
                if n == 11 || n == 12 {
                    assert!(AtomicShares::is_member_of_group(second_group_id, &n));
                }
                if n > 11 {
                    assert!(AtomicShares::is_member_of_group(second_first_group_id, &n));
                }
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
        assert_ok!(AtomicShares::batch_issue_shares(one, 1, 10, genesis));
        // get all registered shares associated with share_id == 0
        // - the first registered id is always 0 by default
        let total = AtomicShares::outstanding_shares(1, 10).unwrap();
        // registration works as expected
        assert_eq!(total, 10);
    });
}

#[test]
fn share_reservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
        let prefix_key = ShareGroup::new(1, 1);
        let profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let first_times_reserved = profile.times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_reserved, 1);
        assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
        let second_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let second_times_reserved = second_profile.times_reserved();
        assert_eq!(second_times_reserved, 2);
        let mut n = 0u32;
        while n < 20 {
            assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
            n += 1;
        }
        let n_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let n_times_reserved = n_profile.times_reserved();
        assert_eq!(n_times_reserved, 22);

        // check same logic with another member of the first group
        assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
        let a_profile = AtomicShares::profile(prefix_key, 2).unwrap();
        let a_first_times_reserved = a_profile.times_reserved();
        // // check that method calculates correctly
        assert_eq!(a_first_times_reserved, 1);
        assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
        let a_second_profile = AtomicShares::profile(prefix_key, 2).unwrap();
        let a_second_times_reserved = a_second_profile.times_reserved();
        assert_eq!(a_second_times_reserved, 2);
        let mut a_n = 0u32;
        while a_n < 20 {
            assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 2));
            a_n += 1;
        }
        let a_n_profile = AtomicShares::profile(prefix_key, 2).unwrap();
        let a_n_times_reserved = a_n_profile.times_reserved();
        assert_eq!(a_n_times_reserved, 22);
    });
}

#[test]
fn share_unreservation() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        assert_ok!(AtomicShares::reserve_shares(one.clone(), 1, 1, 1));
        let prefix_key = ShareGroup::new(1, 1);
        let profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let first_times_reserved = profile.times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_reserved, 1);
        assert_ok!(AtomicShares::unreserve_shares(one.clone(), 1, 1, 1));
        let un_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let first_times_un_reserved = un_profile.times_reserved();
        // // check that method calculates correctly
        assert_eq!(first_times_un_reserved, 0);
    });
}

#[test]
fn share_lock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = ShareGroup::new(1, 1);
        let profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(AtomicShares::lock_shares(one.clone(), 1, 1, 1));
        let locked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
    });
}

#[test]
fn share_unlock() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = ShareGroup::new(1, 1);
        let profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let unlocked = profile.is_unlocked();
        assert_eq!(unlocked, true);
        assert_ok!(AtomicShares::lock_shares(one.clone(), 1, 1, 1));
        let locked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let locked = !locked_profile.is_unlocked();
        assert_eq!(locked, true);
        assert_ok!(AtomicShares::unlock_shares(one.clone(), 1, 1, 1));
        let unlocked_profile = AtomicShares::profile(prefix_key, 1).unwrap();
        let is_unlocked = unlocked_profile.is_unlocked();
        assert_eq!(is_unlocked, true);
    });
}

#[test]
fn share_issuance() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = ShareGroup::new(1, 1);
        let pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
        let pre_shares = pre_profile.total();

        assert_eq!(pre_shares, 10);
        // issue 10 new shares to 7
        assert_ok!(AtomicShares::issue_shares(one.clone(), 1, 1, 10, 10));

        let post_profile = AtomicShares::profile(prefix_key, 10).unwrap();
        let post_shares = post_profile.total();

        assert_eq!(post_shares, 20);
    });
}

#[test]
fn share_burn() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let prefix_key = ShareGroup::new(1, 1);
        let pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
        let pre_shares = pre_profile.total();

        assert_eq!(pre_shares, 10);
        // issue 10 new shares to 10
        assert_ok!(AtomicShares::issue_shares(one.clone(), 1, 1, 10, 10));

        let pre_pre_profile = AtomicShares::profile(prefix_key, 10).unwrap();
        let pre_pre_shares = pre_pre_profile.total();

        assert_eq!(pre_pre_shares, 20);
        // burn 10 new shares for 10
        assert_ok!(AtomicShares::burn_shares(one.clone(), 1, 1, 10, 10));
        let post_profile = AtomicShares::profile(prefix_key, 10).unwrap();
        let post_shares = post_profile.total();

        assert_eq!(post_shares, 10);
    });
}

// add subsupervisor governance test
