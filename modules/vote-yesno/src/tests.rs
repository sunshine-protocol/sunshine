use super::*;
use frame_support::assert_ok; // assert_err, assert_noop
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    shares_atomic::GenesisConfig::<Test> {
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
fn votes_created_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_vote(
            one,
            1,
            1,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));

        // get vote state
        let prefix_key = OrgSharePrefixKey::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix_key, 1).unwrap();
        // verify expected defaults -- TODO: verify other fields
        assert_eq!(vote_state.turnout, 0);
        // get vote outcome
        let vote_outcome = VoteYesNo::vote_outcome(prefix_key, 1).unwrap();
        // check that it is in the voting stage
        assert_eq!(vote_outcome, Outcome::Voting);

        // TODO: check share reservation amounts (none should be free)
    });
}

#[test]
fn votes_apply_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_vote(
            one.clone(),
            1,
            1,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));
        assert_ok!(VoteYesNo::submit_vote(
            one,
            1,
            1,
            1,
            1,
            VoterYesNoView::InFavor,
            None
        ));

        // get vote state
        let prefix = OrgSharePrefixKey::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults -- TODO: verify other fields
        assert_eq!(vote_state.turnout, 10);
    });
}
