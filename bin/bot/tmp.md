// -> add optional github issue param to bounty event emission
// async fn subscribe_to_bounty_post_event() {
//     env_logger::try_init().ok();
//     let alice = PairSigner::new(AccountKeyring::Alice.pair());
//     let bob = AccountKeyring::Bob.to_account_id();
//     let (client, _) = test_client().await;
//     let sub = client.subscribe_events().await.unwrap();
//     let mut decoder = EventsDecoder::<TestRuntime>::new(client.metadata().clone());
//     decoder.with_balances();

//     let mut sub = EventSubscription::<TestRuntime>::new(sub, decoder);
//     sub.filter_event::<TransferEvent<_>>();
//     client.transfer(&alice, &bob, 10_000).await.unwrap();
//     let raw = sub.next().await.unwrap().unwrap();
//     let event = TransferEvent::<TestRuntime>::decode(&mut &raw.data[..]).unwrap();
//     assert_eq!(
//         event,
//         TransferEvent {
//             from: alice.account_id().clone(),
//             to: bob.clone(),
//             amount: 10_000,
//         }
//     );
// }