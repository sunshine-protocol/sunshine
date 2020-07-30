![sunset](https://user-images.githubusercontent.com/741807/81438174-95909f00-916c-11ea-9bb2-ca677781069f.jpg)
> "Over time, all the components of the DAO are likely to be upgraded using its own mechanisms...Given the high requirements for stability, **self-improvement will be critical to the survival of any DAO-based democratic system.**" ~[DAOs, Democracy and Governance](http://merkle.com/papers/DAOdemocracyDraft.pdf) by Ralph Merkle

This is DAO-chain implementation with Substrate. It is experimental and the design intends to focus on short-term developer contracts. It may become something like Gitcoin + Aragon with organizations using the platform to post bounties and teams using the platform to pursue bounties and eventually even raise funds for their own projects.

## Runtime Logic
> `./pallets/*`

### Org

Every organization encodes membership with ownership expressed as `Vec<(AccountId, Shares)>`. Each org has an `OrgId`, which is used to establish ownership of state associated with the group.

Every group has a sudo `Option<AccountId>`. The intention is that this position will be a representative selected by the group to _keep things moving_, but their selection will be easily revocable.

### Vote

Provides functionality for dispatching votes with the given group's membership as the electorate. These votes may be weighted by the group's ownership or 1 account 1 vote.

### Court

Uses `vote` to dispatch votes to resolve disputes between two parties. Like insurance, one party might only agree to enter into an external contract with the other party if they agree to stake collateral and forfeit that collateral in the event that the dispatched vote resolves against them.

### Donate

Allows any `AccountId` to transfer funds to an `OrgId` such that the funds are distributed to the members of the group in proportion to their ownership in the group.

### Bank

Enables orgs to create joint bank accounts with spends governed by group votes.

### Bounty

Allows individual accounts to post bounties and govern/supervise execution. Supports outside contributions.

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
