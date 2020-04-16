# sunshine-subxt

fork of [paritytech/substrate-subxt](https://github.com/paritytech/substrate-subxt) that works with [web3garden/sunshine-node](https://github.com/web3garden/sunshine-node)

## instructions

2. Compile the node in release mode
```bash
# from root of the sunshine-node
$ cargo build --release
```
3. Run the single chain deveopment mode for the node
```bash
# from root of the sunshine-node
$ ./target/release/sunshine-node --dev
```
4. In a separate terminal, run this repo's `reserve_shares_and_watch` example
```bash
# in `./client` (this folder)
cargo run --example reserve_shares_and_watch
```
5. The output looks like
```bash
# in `./client` (this folder)
Account d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d (5GrwvaEF...) reserved 1 shares with share id 1 for organization id 1
```
