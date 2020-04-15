# sunshine-subxt

fork of [paritytech/substrate-subxt](https://github.com/paritytech/substrate-subxt) that works with [web3garden/sunshine-node](https://github.com/web3garden/sunshine-node)

## instructions

1. Clone this repo and the [node](https://github.com/web3garden/sunshine-node)

2. Compile the node in release mode
```bash
$ cargo build --release
```
3. Run the single chain deveopment mode for the node
```bash
$ ./target/release/sunshine-node --dev
```
4. In a separate terminal, run this repo's `reserve_shares_and_watch` example
```bash
cargo run --example reserve_shares_and_watch
```
5. The output looks like
```bash
Account d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d (5GrwvaEF...) reserved 1 shares with share id 1 for organization id 1
```
