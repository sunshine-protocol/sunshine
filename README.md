# ⓢⓤⓝⓢⓗⓘⓝⓔ 🌞

**sunshine** is a fund coordination DAO that fosters nuanced governance over pooled resources. The architecture is built to evolve and encourages **self-improvement**.

* [overview](#over)
* [runtime architecture](#runtime)
* [build instructions](#build)

## overview <a name = "over"></a>

> "Over time, all the components of the DAO are likely to be upgraded using its own mechanisms...Given the high requirements for stability, **self-improvement will be critical to the survival of any DAO-based democratic system.**" ~[DAOs, Democracy and Governance]() by Ralph Merkle

With continuous self-improvement in mind, sunshine's mechanism design aspires to be
1. **accessible**: usability determines the diversity of users as well as rate of adoption
2. **forkable**: modularity and extensibility encourage granular configuration based on use case
3. **dynamic**: unambiguous on-chain runtime upgrades allow the system to adapt and evolve
4. **human-centered**: fairness and sustainability are paramount in all design decisions

Piggybacking on Substrate’s on-chain upgrades, sunshine introduces **metagovernance** for elegant upgrade paths. Stakeholders can vote on any changes to the application, including the terms of the vote itself. This increased flexibility allows the application to **evolve around user requirements** and update features without hard forks.

Straightforward upgrades make applications built on the blockchain more suitable to tackle real-world problems, many of which maintain **dynamic** stakeholder sets with **dynamic** relationships. Indeed, static mechanism design will quickly become obsolete in the context of a world that is changing faster every day.

## runtime architecture <a name = "runtime"></a>

> minimal, single-file implementations are maintained in [dao-templates](https://github.com/web3garden/dao-templates)

the [runtime](./runtime/) configures four [modules](./modules): 
* [membership]()
* [voting]()
* [fund]()
* [committee]()

## sovereign::chain, para::{thread, chain}?

**sunshine** comprises of a set of modules that can be included in any parachain and/or parathread runtime to facilitate fund coordination amongst a defined stakeholder set

When designing **sunshine**, the initial motivating use case is the Polkadot treasury; [Kusama's treasury governance](https://medium.com/polkadot-network/kusama-rollout-and-governance-31eb18041044) does not support nuanced fund management. While it allows for voting on donations (*with a binary yes/no outcome*), the current design does not support investments or targeted liquidity provision. Additional experimental features are discussed in [modules](./modules/README#y), but a priority is the incorporation of forward guidance as detailed in [web3garden/monetary-futarchy](https://github.com/web3garden/monetary-futarchy).

## build instructions <a name = "build"></a>

Install Rust:

```bash
curl https://sh.rustup.rs -sSf | sh
```

Install required tools:

```bash
./scripts/init.sh
```

Build Wasm and native code:

```bash
cargo build
```

### single node development chain

You can start a development chain with:

```bash
cargo run -- --dev
```

Detailed logs may be shown by running the node with the following environment variables set: `RUST_LOG=debug RUST_BACKTRACE=1 cargo run -- --dev`.

### multi-node local testnet

If you want to see the multi-node consensus algorithm in action locally, then you can create a local testnet with two validator nodes for Alice and Bob, who are the initial authorities of the genesis chain that have been endowed with testnet units.

Optionally, give each node a name and expose them so they are listed on the Polkadot [telemetry site](https://telemetry.polkadot.io/#/Local%20Testnet).

You'll need two terminal windows open.

We'll start Alice's substrate node first on default TCP port 30333 with her chain database stored locally at `/tmp/alice`. The bootnode ID of her node is `QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`, which is generated from the `--node-key` value that we specify below:

```bash
cargo run -- \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

In the second terminal, we'll start Bob's substrate node on a different TCP port of 30334, and with his chain database stored locally at `/tmp/bob`. We'll specify a value for the `--bootnodes` option that will connect his node to Alice's bootnode ID on TCP port 30333:

```bash
cargo run -- \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR \
  --chain=local \
  --bob \
  --port 30334 \
  --telemetry-url ws://telemetry.polkadot.io:1024 \
  --validator
```

Additional CLI usage options are available and may be shown by running `cargo run -- --help`.
