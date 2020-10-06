## subxt

[`substrate-subxt`](https://github.com/paritytech/substrate-subxt) is a rust substrate client built to interface with the substrate chain. It provides light client support, making it possible to work with untrusted substrate nodes. 

It is unique in it's support for writing integration tests, by replacing the light client with a full node. This functionality is demonstrated in `sunshine-keybase`. 

To see these tests in action, [clone the repo](https://github.com/sunshine-protocol/sunshine-keybase) and run the following commands

```bash
$ git clone https://github.com/sunshine-protocol/sunshine-keybase
$ cd sunshine-keybase && cd chain/client
$ cargo test --release
```

Here is an example of expected output. `UnknownSubscriptionId` errors are *[usually OK](https://github.com/paritytech/substrate-subxt/issues/94)*.

```bash
running 3 tests
[2020-10-06T18:28:36Z ERROR jsonrpsee::client] Client Error: UnknownSubscriptionId
[2020-10-06T18:28:36Z ERROR jsonrpsee::client] Client Error: UnknownSubscriptionId
[2020-10-06T18:28:42Z ERROR jsonrpsee::client] Client Error: UnknownSubscriptionId
test tests::test_sync ... ok
test tests::test_concurrent ... ok
test tests::test_chain ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests sunshine-chain-client

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

There are more client integration tests in `identity/client`.

```bash
➜  client git:(master) cart --release
   Compiling sunshine-identity-client v0.2.3 (/Users/4meta5/sunshine-protocol/sunshine-keybase/identity/client)
    Finished release [optimized] target(s) in 29.16s
     Running /Users/4meta5/sunshine-protocol/sunshine-keybase/target/release/deps/sunshine_identity_client-d858ac81e954b312

running 4 tests
test utils::tests::parse_identifer ... ok
test client::tests::provision_device ... ok
test client::tests::change_password ... ok
test client::tests::prove_identity ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

   Doc-tests sunshine-identity-client

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```