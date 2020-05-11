use super::*;
// use frame_support::assert_err;
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
    t.into()
}

#[test]
fn test_flat_share_group_trait() {
    new_test_ext().execute_with(|| {
        let expected_vec = vec![11u64, 8u64, 9u64, 10u64, 12u64];
        assert_eq!(
            Some(expected_vec),
            SharesMembership::get_organization_share_group(1, 2)
        );
    });
}

#[test]
fn test_genesis_with_membership_checks() {
    new_test_ext().execute_with(|| {
        let first_group = UUID2::new(1, 1);
        let second_group = UUID2::new(1, 2);
        let third_group = UUID2::new(1, 3);
        let fifth_group = UUID2::new(1, 5);
        let second_org = UUID2::new(2, 1);
        for i in 1..13 {
            assert!(SharesMembership::is_member_of_group(first_group, &i));
            if i > 9 {
                assert!(SharesMembership::is_member_of_group(fifth_group, &i));
            }
            if i > 7 {
                assert!(SharesMembership::is_member_of_group(second_group, &i));
            }
            if i < 5 {
                assert!(SharesMembership::is_member_of_group(third_group, &i));
            }
            if i < 3 {
                assert!(SharesMembership::is_member_of_group(fifth_group, &i));
            }
            assert!(SharesMembership::is_member_of_group(second_org, &i));
        }
        for j in 13..21 {
            assert!(SharesMembership::is_member_of_group(second_org, &j));
        }
    });
}

// TODO: update with latest permissions traits logic
// #[test]
// fn supervisor_selection_governance_works_as_expected() {
//     // very centralized as is the current design
//     new_test_ext().execute_with(|| {
//         // 1 can assign 7 as the supervisor for this organization
//         let new_supervisor = SharesMembership::swap_sub_supervisor(1, 1, 1, 7).unwrap();
//         let check_new_supervisor = SharesMembership::organization_share_supervisor(1, 1).unwrap();
//         assert_eq!(check_new_supervisor, 7);
//         assert_eq!(check_new_supervisor, new_supervisor);
//         // 7 can assign 9
//         let new_supervisor_seven_to_nine =
//             SharesMembership::swap_sub_supervisor(1, 1, 7, 9).unwrap();
//         let check_new_supervisor_seven_to_nine =
//             SharesMembership::organization_share_supervisor(1, 1).unwrap();
//         assert_eq!(check_new_supervisor_seven_to_nine, 9);
//         assert_eq!(
//             check_new_supervisor_seven_to_nine,
//             new_supervisor_seven_to_nine
//         );
//         // 7 can't assign because 9 has the power
//         assert_err!(
//             SharesMembership::swap_sub_supervisor(1, 1, 7, 11),
//             Error::<Test>::UnAuthorizedRequestToSwapSupervisor
//         );
//         // 1 can reassign to 7 despite not being 9 because it is sudo
//         let new_supervisor_nine_to_seven =
//             SharesMembership::swap_sub_supervisor(1, 1, 1, 7).unwrap();
//         let check_new_supervisor_nine_to_seven =
//             SharesMembership::organization_share_supervisor(1, 1).unwrap();
//         assert_eq!(check_new_supervisor_nine_to_seven, 7);
//         assert_eq!(
//             check_new_supervisor_nine_to_seven,
//             new_supervisor_nine_to_seven
//         );
//     });
// }
