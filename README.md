![sunset](https://user-images.githubusercontent.com/741807/81438174-95909f00-916c-11ea-9bb2-ca677781069f.jpg)
> "Over time, all the components of the DAO are likely to be upgraded using its own mechanisms...Given the high requirements for stability, **self-improvement will be critical to the survival of any DAO-based democratic system.**" ~[DAOs, Democracy and Governance](http://merkle.com/papers/DAOdemocracyDraft.pdf) by Ralph Merkle

This is DAO-chain implementation with Substrate. It is experimental and the design intends to focus on short-term developer contracts. It may become something like Gitcoin + Aragon with organizations using the platform to post bounties and teams using the platform to pursue bounties and eventually even raise funds for their own projects.

## Build

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Initialize your Wasm Build environment:

```bash
./scripts/init.sh
```

Build Wasm and native code:

```bash
cargo build --release
```

## Run Single Node Development Chain

Purge any existing developer chain state:

```bash
./target/release/test-node purge-chain --dev
```

Start a development chain with:

```bash
./target/release/test-node --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.
