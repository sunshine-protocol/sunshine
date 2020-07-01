![sunset](https://user-images.githubusercontent.com/741807/81438174-95909f00-916c-11ea-9bb2-ca677781069f.jpg)
> "Over time, all the components of the DAO are likely to be upgraded using its own mechanisms...Given the high requirements for stability, **self-improvement will be critical to the survival of any DAO-based democratic system.**" ~[DAOs, Democracy and Governance](http://merkle.com/papers/DAOdemocracyDraft.pdf) by Ralph Merkle

## What is This?

Like any other blockchain, this is infrastructure, NOT necessarily a product built on top of infrastructure. It is experimental and the design intends to focus on short-term developer contracts. It could become something like Gitcoin + Aragon with foundations using the platform to post bounties and teams using the platform to pursue bounties and eventually even raise funds for their own projects.

The most useful parts of it right now are the `org` and `vote` modules. I'm pretty happy with how those turned out. The goal was to build something so that I could refer to unweighted (`Vec<AccountId>`) or weighted (`Vec<(AccountId, Shares)>`) groups with a single `OrgId` identifier. Now I pass that identifier around to establish ownership of state associated with the group. The `vote` module allows me to dispatch votes with the given group's membership as the electorate.

`court` uses `vote` to dispatch votes to resolve disputes between two parties. Like insurance, one party might only agree to enter into an external contract with the other party if they agree to stake collateral and forfeit that collateral in the event that the dispatched vote resolves against them.

`bank` is an opinionated design for joint bank accounts. It's a bit complicated and, unfortunately, `bounty` inherits this complexity. I have ideas for improving `bank` so it's more readable and the direction is more clear, but am unsure when I'll get back to it.

## Rust/Substrate Onboarding

To work with this codebase, familiarity with Rust and Substrate is helpful.

New to Rust?
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
./target/release/test-node purge-chain --dev
```

Start a development chain with:

```bash
./target/release/test-node --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.
