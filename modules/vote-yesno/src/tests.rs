use super::*;
use frame_support::{assert_err, assert_ok}; //assert_noop
use mock::*;
use util::traits::ConsistentThresholdStructure;

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
    shares_membership::GenesisConfig::<Test> {
        share_supervisors: Some(vec![(1, 1, 1), (1, 2, 10), (1, 3, 3), (1, 5, 1), (2, 1, 1)]),
        shareholder_membership: Some(vec![
            // org, share, account
            (1, 1, 1, true),
            (1, 1, 2, true),
            (1, 1, 3, true),
            (1, 1, 4, true),
            (1, 1, 5, true),
            (1, 1, 6, true),
            (1, 1, 7, true),
            (1, 1, 8, true),
            (1, 1, 9, true),
            (1, 1, 10, true),
            (1, 1, 11, true),
            (1, 1, 12, true),
            (1, 2, 8, true),
            (1, 2, 9, true),
            (1, 2, 10, true),
            (1, 2, 11, true),
            (1, 2, 12, true),
            (1, 3, 1, true),
            (1, 3, 2, true),
            (1, 3, 3, true),
            (1, 3, 4, true),
            (1, 5, 1, true),
            (1, 5, 2, true),
            (1, 5, 10, true),
            (1, 5, 11, true),
            (1, 5, 12, true),
            (2, 1, 1, true),
            (2, 1, 2, true),
            (2, 1, 3, true),
            (2, 1, 4, true),
            (2, 1, 5, true),
            (2, 1, 6, true),
            (2, 1, 7, true),
            (2, 1, 8, true),
            (2, 1, 9, true),
            (2, 1, 10, true),
            (2, 1, 11, true),
            (2, 1, 12, true),
            (2, 1, 13, true),
            (2, 1, 14, true),
            (2, 1, 15, true),
            (2, 1, 16, true),
            (2, 1, 17, true),
            (2, 1, 18, true),
            (2, 1, 19, true),
            (2, 1, 20, true),
        ]),
    }
    .assimilate_storage(&mut t)
    .unwrap();
    shares_atomic::GenesisConfig::<Test> {
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
fn vote_1p1v_created_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_1p1v_count_threshold_vote(
            one, 1, 1, 3u64, // just requires 3 votes in favor
            0u64,
        ));

        // get vote state
        let prefix_key = UUID2::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix_key, 1).unwrap();
        // verify expected defaults
        assert_eq!(vote_state.turnout(), 0);
        // check that the threshold is of the expected type (percentage)
        assert!(vote_state.threshold().is_count_threshold());
        // get vote outcome
        let vote_outcome = VoteYesNo::vote_outcome(prefix_key, 1).unwrap();
        // check that it is in the voting stage
        assert_eq!(vote_outcome, Outcome::Voting);
    });
}

#[test]
fn vote_1p1v_apply_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        // 1 creates a vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_1p1v_count_threshold_vote(
            one.clone(),
            1,
            1,
            3u64, // just requires 3 votes in favor
            0u64,
        ));

        // 1 votes in favor
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::InFavor,
            None,
            None,
        ));

        // verify expected vote state
        let prefix = UUID2::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(vote_state.turnout(), 1);
        assert_eq!(vote_state.in_favor(), 1);
        assert_eq!(vote_state.against(), 0);

        // 69 cannot vote in favor because it is not in the group
        assert_err!(
            VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                1,
                69,
                VoterYesNoView::InFavor,
                None,
                None
            ),
            Error::<Test>::NotEnoughSignalToVote
        );

        // 2 votes against
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            2,
            VoterYesNoView::Against,
            None,
            None,
        ));

        // verify expected vote state
        let new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_vote_state.turnout(), 2);
        assert_eq!(new_vote_state.in_favor(), 1);
        assert_eq!(new_vote_state.against(), 1);

        // 1 changes their vote to against
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::Against,
            None,
            None,
        ));

        // verify expected vote state
        let new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_vote_state.turnout(), 2);
        assert_eq!(new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_vote_state.against(), 2);

        // 1 changes their vote to abstain
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::Abstain,
            None,
            None,
        ));

        // verify expected vote state
        let new_new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_new_vote_state.turnout(), 2);
        assert_eq!(new_new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_new_vote_state.against(), 1);

        // 2 votes again for against and nothing should change
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            2,
            VoterYesNoView::Against,
            None,
            None
        ));

        // verify expected vote state
        let new_new_new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_new_new_vote_state.turnout(), 2);
        assert_eq!(new_new_new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_new_new_vote_state.against(), 1);
    });
}

#[test]
fn vote_1p1v_threshold_enforced_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        // 1 creates a vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_1p1v_count_threshold_vote(
            one.clone(),
            1,
            1,
            5u64,
            0u64,
        ));

        // let first_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 1, 1));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == first_vote_created));

        // 6 votes allowed 6/10 is the first vote above 50%
        for i in 1..7 {
            // [1, 6] s.t. [] inclusive
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                1,
                i,
                VoterYesNoView::InFavor,
                None,
                None,
            ));
        }
        // 7th isnt allowed because threshold already exceeded when 6 was applied
        assert_err!(
            VoteYesNo::submit_vote(one.clone(), 1, 1, 1, 7, VoterYesNoView::InFavor, None, None),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        // check outcome
        let first_vote_outcome = VoteYesNo::get_vote_outcome(1, 1, 1).unwrap();
        assert_eq!(first_vote_outcome, Outcome::Approved);

        // 1 creates a vote for share group 2 in organization 1
        assert_ok!(VoteYesNo::create_1p1v_count_threshold_vote(
            one.clone(),
            1,
            2,
            1,
            0,
        ));

        // let second_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 2, 1));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == second_vote_created));

        // only 1 && 2 required
        for i in 8..10 {
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                2,
                1,
                i,
                VoterYesNoView::InFavor,
                None,
                None,
            ));
        }
        // 3 is rejected because we already exceed the threshold
        assert_err!(
            VoteYesNo::submit_vote(
                one.clone(),
                1,
                2,
                1,
                11,
                VoterYesNoView::InFavor,
                None,
                None
            ),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        let second_vote_outcome = VoteYesNo::get_vote_outcome(1, 2, 1).unwrap();
        assert_eq!(second_vote_outcome, Outcome::Approved);

        // 1 creates another vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_1p1v_count_threshold_vote(
            one.clone(),
            1,
            1,
            3,
            0,
        ));

        // let third_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 1, 2));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == third_vote_created));

        for i in 1..5 {
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                2,
                i,
                VoterYesNoView::InFavor,
                None,
                None,
            ));
        }
        assert_err!(
            VoteYesNo::submit_vote(one.clone(), 1, 1, 2, 3, VoterYesNoView::InFavor, None, None),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        let third_vote_outcome = VoteYesNo::get_vote_outcome(1, 1, 2).unwrap();
        assert_eq!(third_vote_outcome, Outcome::Approved);
    });
}

#[test]
fn vote_share_weighted_created_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        assert_ok!(VoteYesNo::create_share_weighted_percentage_threshold_vote(
            one,
            1,
            1,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));

        // get vote state
        let prefix_key = UUID2::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix_key, 1).unwrap();
        // verify expected defaults
        assert_eq!(vote_state.turnout(), 0);
        // check that the threshold is of the expected type (percentage)
        assert!(vote_state.threshold().is_percentage_threshold());
        // get vote outcome
        let vote_outcome = VoteYesNo::vote_outcome(prefix_key, 1).unwrap();
        // check that it is in the voting stage
        assert_eq!(vote_outcome, Outcome::Voting);
    });
}

#[test]
fn vote_share_weighted_apply_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        // 1 creates a vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_share_weighted_percentage_threshold_vote(
            one.clone(),
            1,
            1,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));

        // 1 votes in favor
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::InFavor,
            None,
            None,
        ));

        // verify expected vote state
        let prefix = UUID2::new(1, 1);
        let vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(vote_state.turnout(), 10);
        assert_eq!(vote_state.in_favor(), 10);
        assert_eq!(vote_state.against(), 0);

        // 11 cannot vote in favor because it is not in the group
        assert_err!(
            VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                1,
                11,
                VoterYesNoView::InFavor,
                None,
                None
            ),
            Error::<Test>::NotEnoughSignalToVote
        );

        // 2 votes against
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            2,
            VoterYesNoView::Against,
            None,
            None,
        ));

        // verify expected vote state
        let new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_vote_state.turnout(), 20);
        assert_eq!(new_vote_state.in_favor(), 10);
        assert_eq!(new_vote_state.against(), 10);

        // 1 changes their vote to against
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::Against,
            None,
            None,
        ));

        // verify expected vote state
        let new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_vote_state.turnout(), 20);
        assert_eq!(new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_vote_state.against(), 20);

        // 1 changes their vote to abstain
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            1,
            VoterYesNoView::Abstain,
            None,
            None,
        ));

        // verify expected vote state
        let new_new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_new_vote_state.turnout(), 20);
        assert_eq!(new_new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_new_vote_state.against(), 10);

        // 2 votes again for against and nothing should change
        assert_ok!(VoteYesNo::submit_vote(
            one.clone(),
            1,
            1,
            1,
            2,
            VoterYesNoView::Against,
            None,
            None,
        ));

        // verify expected vote state
        let new_new_new_new_vote_state = VoteYesNo::vote_states(prefix, 1).unwrap();
        // verify expected defaults
        assert_eq!(new_new_new_new_vote_state.turnout(), 20);
        assert_eq!(new_new_new_new_vote_state.in_favor(), 0);
        assert_eq!(new_new_new_new_vote_state.against(), 10);
    });
}

#[test]
fn vote_share_weighted_threshold_enforced_correctly() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);

        // 1 creates a vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_share_weighted_percentage_threshold_vote(
            one.clone(),
            1,
            1,
            Permill::from_percent(51),
            Permill::from_percent(10)
        ));

        // let first_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 1, 1));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == first_vote_created));

        // 6 votes allowed 6/10 is the first vote above 50%
        for i in 1..7 {
            // [1, 6] s.t. [] inclusive
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                1,
                i,
                VoterYesNoView::InFavor,
                None,
                None,
            ));
        }
        // threshold exceeded
        assert_err!(
            VoteYesNo::submit_vote(one.clone(), 1, 1, 1, 7, VoterYesNoView::InFavor, None, None),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        // check outcome
        let first_vote_outcome = VoteYesNo::get_vote_outcome(1, 1, 1).unwrap();
        assert_eq!(first_vote_outcome, Outcome::Approved);

        // 1 creates a vote for share group 2 in organization 1
        assert_ok!(VoteYesNo::create_share_weighted_percentage_threshold_vote(
            one.clone(),
            1,
            2,
            Permill::from_percent(33),
            Permill::from_percent(10)
        ));

        // let second_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 2, 1));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == second_vote_created));

        for i in 8..10 {
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                2,
                1,
                i,
                VoterYesNoView::InFavor,
                None,
                None
            ));
        }
        assert_err!(
            VoteYesNo::submit_vote(
                one.clone(),
                1,
                2,
                1,
                11,
                VoterYesNoView::InFavor,
                None,
                None
            ),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        let second_vote_outcome = VoteYesNo::get_vote_outcome(1, 2, 1).unwrap();
        assert_eq!(second_vote_outcome, Outcome::Approved);

        // 1 creates another vote for share group 1 in organization 1
        assert_ok!(VoteYesNo::create_share_weighted_percentage_threshold_vote(
            one.clone(),
            1,
            1,
            Permill::from_percent(33),
            Permill::from_percent(10)
        ));

        // let third_vote_created = TestEvent::vote_yesno(RawEvent::NewVoteStarted(1, 1, 2));
        // assert!(System::events()
        //     .iter()
        //     .any(|a| a.event == third_vote_created));

        for i in 1..5 {
            assert_ok!(VoteYesNo::submit_vote(
                one.clone(),
                1,
                1,
                2,
                i,
                VoterYesNoView::InFavor,
                None,
                None
            ));
        }
        assert_err!(
            VoteYesNo::submit_vote(one.clone(), 1, 1, 2, 3, VoterYesNoView::InFavor, None, None),
            Error::<Test>::CanOnlyVoteinVotingOutcome
        );
        let third_vote_outcome = VoteYesNo::get_vote_outcome(1, 1, 2).unwrap();
        assert_eq!(third_vote_outcome, Outcome::Approved);
    });
}
