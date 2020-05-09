use super::*;
use frame_support::assert_err;
use mock::*;

fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    GenesisConfig::<Test> {
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
    t.into()
}

#[test]
fn test_genesis() {
    new_test_ext().execute_with(|| {
        for i in 1..21 {
            assert!(Membership::is_member_of_group(5, &i));
            if i < 13 {
                assert!(Membership::is_member_of_group(1, &i));
                if i > 7 {
                    assert!(Membership::is_member_of_group(2, &i));
                }
                if i < 5 {
                    assert!(Membership::is_member_of_group(3, &i));
                }
                if i < 3 || i > 9 {
                    assert!(Membership::is_member_of_group(4, &i));
                }
            }
        }
        assert_eq!(Membership::get_size_of_group(1), 12);
        assert_eq!(Membership::get_size_of_group(2), 5);
        assert_eq!(Membership::get_size_of_group(3), 4);
        assert_eq!(Membership::get_size_of_group(4), 5);
        assert_eq!(Membership::get_size_of_group(5), 20);
    });
}

#[test]
fn supervisor_selection_governance_works_as_expected() {
    // very centralized as is the current design
    new_test_ext().execute_with(|| {
        // 1 can assign 7 as the supervisor for this organization
        let new_supervisor = Membership::swap_supervisor(1, 1, 7).unwrap();
        let check_new_supervisor = Membership::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor, 7);
        assert_eq!(check_new_supervisor, new_supervisor);
        // 7 can assign 9
        let new_supervisor_seven_to_nine = Membership::swap_supervisor(1, 7, 9).unwrap();
        let check_new_supervisor_seven_to_nine = Membership::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor_seven_to_nine, 9);
        assert_eq!(
            check_new_supervisor_seven_to_nine,
            new_supervisor_seven_to_nine
        );
        // 7 can't assign because 9 has the power
        assert_err!(
            Membership::swap_supervisor(1, 7, 11),
            Error::<Test>::UnAuthorizedRequestToSwapSupervisor
        );
        // 1 can reassign to 7 despite not being 9 because it is sudo
        let new_supervisor_nine_to_seven = Membership::swap_supervisor(1, 1, 7).unwrap();
        let check_new_supervisor_nine_to_seven = Membership::organization_supervisor(1).unwrap();
        assert_eq!(check_new_supervisor_nine_to_seven, 7);
        assert_eq!(
            check_new_supervisor_nine_to_seven,
            new_supervisor_nine_to_seven
        );
    });
}
