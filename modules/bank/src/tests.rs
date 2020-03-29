use super::*;
use frame_support::{assert_err, assert_ok}; // assert_err, assert_noop
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
    GenesisConfig::<Test> { omnipotent_key: 1 }
        .assimilate_storage(&mut t)
        .unwrap();
    t.into()
}

#[test]
fn organization_registration() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis_allocation = vec![(1, 10), (2, 20), (3, 30), (9, 1), (10, 2)];
        let constitution: &[u8] = b"my rule here";
        use sp_core::H256;
        use sp_runtime::traits::{BlakeTwo256, Hash};
        let constitutional_hash: H256 = BlakeTwo256::hash(constitution);
        // no organizations before registration call
        assert_eq!(Bank::organization_count(), 0);
        // next line is registration call
        assert_ok!(Bank::register_organization(
            one,
            None,
            1,
            constitutional_hash,
            3,
            genesis_allocation
        ));
        // check organization count changed as expected
        assert_eq!(Bank::organization_count(), 1);
        // summoner is the summoner as expected
        let supervisor = Bank::organization_supervisor(1).unwrap();
        assert_eq!(supervisor, 1);
        // event emitted as expected
        let expected_event = TestEvent::bank(RawEvent::NewOrganizationRegistered(1, 1, 3));
        assert!(System::events().iter().any(|a| a.event == expected_event));
        // TODO: verify all storage changes
        // TODO: error paths and correct errors
    });
}

#[test]
fn supervisor_selection_governance_works_as_expected() {
    // very centralized as is the current design
    new_test_ext().execute_with(|| {
        // 1 can assign 7 as the supervisor for this organization
        let new_supervisor = Bank::swap_supervisor(1, 1, 7).unwrap();
        let check_new_supervisor = Bank::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor, 7);
        assert_eq!(check_new_supervisor, new_supervisor);
        // 7 can assign 9
        let new_supervisor_seven_to_nine = Bank::swap_supervisor(1, 7, 9).unwrap();
        let check_new_supervisor_seven_to_nine = Bank::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor_seven_to_nine, 9);
        assert_eq!(
            check_new_supervisor_seven_to_nine,
            new_supervisor_seven_to_nine
        );
        // 7 can't assign because 9 has the power
        assert_err!(
            Bank::swap_supervisor(1, 7, 11),
            Error::<Test>::UnAuthorizedRequestToSwapSupervisor
        );
        // 1 can reassign to 7 despite not being 9 because it is sudo
        let new_supervisor_nine_to_seven = Bank::swap_supervisor(1, 1, 7).unwrap();
        let check_new_supervisor_nine_to_seven = Bank::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor_nine_to_seven, 7);
        assert_eq!(
            check_new_supervisor_nine_to_seven,
            new_supervisor_nine_to_seven
        );
    });
}

#[test]
fn change_value_constitution() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis_allocation = vec![(1, 10), (2, 20), (3, 30), (9, 1), (10, 2)];
        use sp_core::H256;
        use sp_runtime::traits::{BlakeTwo256, Hash};
        let constitution: &[u8] = b"my rule here";
        let new_constitution: &[u8] = b"newer rule here";
        let even_more_new_constitution: &[u8] = b"even more new rule here";
        let newest_constitution: &[u8] = b"newest rule here";
        let constitutional_hash: H256 = BlakeTwo256::hash(constitution);
        let new_constitutional_hash: H256 = BlakeTwo256::hash(new_constitution);
        let even_more_new_constitutional_hash: H256 = BlakeTwo256::hash(even_more_new_constitution);
        let newest_constitutional_hash: H256 = BlakeTwo256::hash(newest_constitution);
        // can't update without a registered organization
        assert_err!(
            Bank::update_value_constitution(one.clone(), 1, new_constitutional_hash),
            Error::<Test>::NoExistingValueConstitution
        );
        // properly register the organization first
        assert_ok!(Bank::register_organization(
            one.clone(),
            None,
            1,
            constitutional_hash,
            3,
            genesis_allocation
        ));
        // now update the constitution from sudo
        assert_ok!(Bank::update_value_constitution(
            one.clone(),
            1,
            new_constitutional_hash
        ));
        // compare it with the storage item
        let current_value_constitution = Bank::value_constitution(1).unwrap();
        assert_eq!(current_value_constitution, new_constitutional_hash);
        // assign a different supervisor
        let expected_seven = Bank::swap_supervisor(1, 1, 7).unwrap();
        assert_eq!(expected_seven, 7);
        // supervisor updates the value constitution
        let seven = Origin::signed(7);
        assert_ok!(Bank::update_value_constitution(
            seven,
            1,
            even_more_new_constitutional_hash
        ));
        let new_current_value_constitution = Bank::value_constitution(1).unwrap();
        assert_eq!(
            new_current_value_constitution,
            even_more_new_constitutional_hash
        );
        // sudo can still update the value constitution
        assert_ok!(Bank::update_value_constitution(
            one,
            1,
            newest_constitutional_hash
        ));
        let most_current_value_constitution = Bank::value_constitution(1).unwrap();
        assert_eq!(most_current_value_constitution, newest_constitutional_hash);
    });
}

#[test]
fn share_registration_in_organization() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis_allocation = vec![(1, 10), (2, 20), (3, 30), (9, 1), (10, 2)];
        assert_err!(
            Bank::register_shares_in_organization(one.clone(), 1, 4, genesis_allocation.clone()),
            Error::<Test>::NoRegisteredOrganizationWithIDProvided
        );
        let constitution: &[u8] = b"my rule here";
        use sp_core::H256;
        use sp_runtime::traits::{BlakeTwo256, Hash};
        let constitutional_hash: H256 = BlakeTwo256::hash(constitution);
        // registration call
        assert_ok!(Bank::register_organization(
            one.clone(),
            None,
            1,
            constitutional_hash,
            3,
            genesis_allocation.clone(),
        ));
        assert_ok!(Bank::register_shares_in_organization(
            one,
            1,
            4,
            genesis_allocation.clone()
        ));
        // TODO: check to see if the shares module reflects this new registration
        // TODO: check event emittance
    });
}

#[test]
fn most_basic_vote_requirements_setting_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let required_share_voting_groups = vec![1, 2];
        assert_err!(
            Bank::set_most_basic_vote_requirements(
                one.clone(),
                1,
                ProposalType::ExecutiveMembership,
                required_share_voting_groups.clone()
            ),
            Error::<Test>::NoRegisteredOrganizationWithIDProvided
        );
        let genesis_allocation = vec![(1, 10), (2, 20), (3, 30), (9, 1), (10, 2)];
        let constitution: &[u8] = b"my rule here";
        use sp_core::H256;
        use sp_runtime::traits::{BlakeTwo256, Hash};
        let constitutional_hash: H256 = BlakeTwo256::hash(constitution);
        // registration call
        assert_ok!(Bank::register_organization(
            one.clone(),
            None,
            1,
            constitutional_hash,
            3,
            genesis_allocation.clone(),
        ));
        assert_ok!(Bank::set_most_basic_vote_requirements(
            one.clone(),
            1,
            ProposalType::ExecutiveMembership,
            required_share_voting_groups.clone()
        ));
        // check that storage reflects recent change
        let share_approval_order = Bank::proposal_default_share_approval_order_for_organization(
            1,
            ProposalType::ExecutiveMembership,
        )
        .unwrap();
        assert_eq!(required_share_voting_groups, share_approval_order);
    });
}

#[test]
fn default_build_sequence_works() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let genesis_allocation = vec![(1, 10), (2, 20), (3, 30), (9, 1), (10, 2)];
        let constitution: &[u8] = b"my rule here";
        use sp_core::H256;
        use sp_runtime::traits::{BlakeTwo256, Hash};
        let constitutional_hash: H256 = BlakeTwo256::hash(constitution);
        // no organizations before registration call
        assert_eq!(Bank::organization_count(), 0);
        // next line is registration call
        assert_ok!(Bank::register_organization(
            one.clone(),
            // no supervisor
            None,
            1,
            constitutional_hash,
            3,
            genesis_allocation.clone()
        ));
        assert_ok!(Bank::register_shares_in_organization(
            one.clone(),
            1,
            4,
            genesis_allocation.clone()
        ));
        assert_ok!(
            Bank::set_organization_share_id_proposal_type_default_threshold(
                one.clone(),
                1,
                3,
                ProposalType::ExecutiveMembership,
                Permill::from_percent(51),
                Permill::from_percent(10),
            )
        );
        assert_ok!(
            Bank::set_organization_share_id_proposal_type_default_threshold(
                one.clone(),
                1,
                4,
                ProposalType::ExecutiveMembership,
                Permill::from_percent(51),
                Permill::from_percent(10),
            )
        );
        let ordered_share_ids = vec![3, 4];
        assert_ok!(Bank::set_most_basic_vote_requirements(
            one.clone(),
            1,
            ProposalType::ExecutiveMembership,
            ordered_share_ids,
        ));
        // if this passes, then the above scaffolding works
        assert_ok!(Bank::make_proposal(
            one.clone(),
            1,
            ProposalType::ExecutiveMembership,
            None
        ));
    });
}

#[test]
fn vote_scheduling_basic_runthrough() {
    // probably will need a `run_to` method for running `OnFinalize` in system context
    new_test_ext().execute_with(|| {});
}
