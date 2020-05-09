use super::*;
use frame_support::{assert_err, assert_ok};
use mock::*;
use rand::{rngs::OsRng, RngCore};

pub fn random(output_len: usize) -> Vec<u8> {
    let mut buf = vec![0u8; output_len];
    OsRng.fill_bytes(&mut buf);
    buf
}

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
    t.into()
}

#[test]
fn create_petition_happy_path() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let ten = Origin::signed(10);
        let new_topic = random(10);
        assert_eq!(VotePetition::petition_id_counter(1, 1), 0);
        // auth only allows sudo or organization supervisor
        assert_err!(
            VotePetition::create_petition(
                ten,
                1,
                1,
                None,
                new_topic.clone(),
                4,
                None,
                None,
                Some(None)
            ),
            Error::<Test>::NotAuthorizedToCreatePetition
        );
        assert_eq!(VotePetition::petition_id_counter(1, 1), 0);
        assert_ok!(VotePetition::create_petition(
            one,
            1,
            1,
            None,
            new_topic.clone(),
            4,
            None,
            None,
            Some(None)
        ));
        assert_eq!(VotePetition::petition_id_counter(1, 1), 1);
        // let petition_started =
        //     TestEvent::vote_petition(RawEvent::NewPetitionStarted(1, 1, 1, 1, true));
        // assert!(System::events().iter().any(|a| a.event == petition_started));
        let prefix = UUID3::new(1, 1, 1);
        for i in 1u64..13u64 {
            // check that everyone in the share group received veto power by default
            assert!(VotePetition::veto_power(prefix, &i).is_some());
        }

        let new_petition_state = PetitionState::new(new_topic, 4, None, 12, None).unwrap();
        let prefix = UUID2::new(1, 1);
        assert_eq!(
            new_petition_state,
            VotePetition::petition_states(prefix, 1).unwrap()
        );
    });
}

#[test]
fn test_getter_of_those_empowered_with_veto() {
    new_test_ext().execute_with(|| {
        let one = Origin::signed(1);
        let new_topic = random(10);
        assert_ok!(VotePetition::create_petition(
            one,
            1,
            1,
            None,
            new_topic.clone(),
            4,
            None,
            None,
            Some(None)
        ));
        let vetoer_group = VotePetition::get_those_empowered_with_veto(1, 1, 1);
        let empty_group = VotePetition::get_those_empowered_with_veto(1, 1, 2);
        assert!(vetoer_group.is_some()); // TODO: could check that this is the same as (1, 1) share group
        assert!(empty_group.is_none());
    });
}
