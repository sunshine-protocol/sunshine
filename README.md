# sunshine node

> "Over time, all the components of the DAO are likely to be upgraded using its own mechanisms...Given the high requirements for stability, **self-improvement will be critical to the survival of any DAO-based democratic system.**" ~[DAOs, Democracy and Governance](http://merkle.com/papers/DAOdemocracyDraft.pdf) by Ralph Merkle

## Onboarding

Interested in contributing and new to Rust?
- [Rust Book](https://doc.rust-lang.org/book/index.html)
- [More Rust Learning Resources](https://github.com/4meta5/learning-rust)

Building a simple CLI tool is a nice first Rust project
- [Rust CLI Book](https://rust-cli.github.io/book/index.html)

To get started with [Substrate](https://github.com/paritytech/substrate)
- [Substrate Recipes](https://github.com/substrate-developer-hub/recipes)

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
./target/release/node-template purge-chain --dev
```

Start a development chain with:

```bash
./target/release/node-template --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.