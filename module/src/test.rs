

#[cfg(test)]
mod tests {
    use crate::tests::Origin;
    use crate::tests::*; // {Call, Event as OuterEvent}
                         // use runtime_primitives::{
                         //     testing::Header,
                         //     traits::{BlakeTwo256, IdentityLookup, OnFinalize},
                         // };
    use support::{assert_err, assert_noop, assert_ok}; // dispatch::Result, Hashable
    use system::ensure_signed; // there must be a better way of getting AccountId

    #[test]
    fn basic_setup_works() {
        with_externalities(&mut make_test_ext(), || {
            assert_eq!(DAO::pot(), 0);
            assert_eq!(DAO::total_proposals(), 0);
        });
    }

    #[test]
    fn join_works() {
        with_externalities(&mut make_test_ext(), || {
            // test that join takes 10 from balance
            assert_ok!(DAO::join(Origin::signed(0)));
            assert_eq!(Balances::free_balance(&0), 90);
            // get address for checking membership
            let who = ensure_signed(Origin::signed(0)).expect("smh^smh");
            assert!(DAO::is_member(&who));; // how do I get the accountId
                                            // join request from existing member should fail
            assert_noop!(
                DAO::join(Origin::signed(0)),
                "new member is already a member"
            );
            // (3, 9) can't join because 9 < 10 (and 10 is EntryFee)
            assert_noop!(DAO::join(Origin::signed(3)), "Not rich enough to join ;(");
        });
    }

    #[test]
    fn exit_works() {
        with_externalities(&mut make_test_ext(), || {
            // join to exit immediately after
            assert_ok!(DAO::join(Origin::signed(0)));
            // exit should work
            assert_ok!(DAO::exit(Origin::signed(0)));
            // exit for non-member should not work
            assert_noop!(
                DAO::exit(Origin::signed(1)),
                "exiting member must be a member"
            );
        });
    }

    #[test]
    fn propose_works() {
        with_externalities(&mut make_test_ext(), || {
            // nonmember propose fails
            assert_noop!(
                DAO::propose(Origin::signed(0), 10, 3),
                "proposer must be a member to make a proposal"
            );
            // join to add 10 to the treasury
            assert_ok!(DAO::join(Origin::signed(0)));
            // proposal outweighs DAO's funds
            assert_noop!(
                DAO::propose(Origin::signed(0), 11, 3),
                "not enough funds in the DAO to execute proposal"
            );
            // 10 + 10 = 20
            assert_ok!(DAO::join(Origin::signed(1)));
            // proposal should work
            assert_ok!(DAO::propose(Origin::signed(0), 11, 3));
            assert_eq!(DAO::total_proposals(), 1);
            // 100 - EntryFee(10) - ProposalBond(2) = 88
            assert_eq!(Balances::free_balance(&0), 88);
            // proposal can't be done without proposal bond
            assert_ok!(DAO::join(Origin::signed(4)));
            assert_noop!(
                DAO::propose(Origin::signed(4), 10, 3),
                "Proposer's balance too low"
            );
        });
    }

    #[test]
    fn vote_works() {
        with_externalities(&mut make_test_ext(), || {
            // nonmember can't vote
            assert_noop!(
                DAO::vote(Origin::signed(0), 1, true),
                "voter must be a member to approve/deny a proposal"
            );
            // join, join
            assert_ok!(DAO::join(Origin::signed(0)));
            assert_ok!(DAO::join(Origin::signed(1)));
            // can't vote on nonexistent proposal
            assert_noop!(DAO::vote(Origin::signed(1), 1, true), "proposal must exist");
            // make proposal for voting
            assert_ok!(DAO::propose(Origin::signed(0), 11, 3));
            assert_eq!(DAO::total_proposals(), 1);
            // vote for member works
            assert_ok!(DAO::vote(Origin::signed(1), 1, true));
            // can't duplicate vote
            // assert_noop!(DAO::vote(Origin::signed(1), 0, true), "duplicate vote"); // doesn't really work
            // can switch vote
            assert_ok!(DAO::vote(Origin::signed(1), 1, false));
            // can't duplicate vote
            // assert_noop!(DAO::vote(Origin::signed(1), 0, false), "duplicate vote"); // doesn't really work
        });
    }
}
