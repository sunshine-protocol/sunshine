use super::*;
use frame_support::assert_ok; // assert_err, assert_noop
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    shares::GenesisConfig::<Test> {
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
fn votes_created_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_default_vote(
            one,
            ProposalType::ExecutiveMembership,
            1,
            2,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));

        // get vote state
        let vote_state = VoteYesNo::vote_states(1).unwrap();
        // verify expected defaults -- TODO: verify other fields
        assert_eq!(vote_state.turnout, 0);
        // get vote outcome
        let vote_outcome = VoteYesNo::vote_outcome(1).unwrap();
        // check that it is in the voting stage
        assert_eq!(vote_outcome, Outcome::Voting);

        // TODO: check share reservation amounts (none should be free)
    });
}

#[test]
fn votes_apply_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_default_vote(
            one.clone(),
            ProposalType::ExecutiveMembership,
            1,
            2,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));
        assert_ok!(VoteYesNo::vote(one, 1, VoterYesNoView::InFavor, None));

        // get vote state
        let vote_state = VoteYesNo::vote_states(1).unwrap();
        // verify expected defaults -- TODO: verify other fields
        assert_eq!(vote_state.turnout, 5);
    });
}
