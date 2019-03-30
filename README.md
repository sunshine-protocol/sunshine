# Molochameleon DAO
> *[moloch moloch moloch molochameleon](https://www.youtube.com/watch?v=JmcA9LIIXWw)*

**Molochameleon** is a minimally viable DAO on Substrate inspired by **[Moloch](https://github.com/MolochVentures/moloch)**. 

**This code is untested and should NOT be used in production**

Molochameleon in its current iteration serves to demonstrate patterns common to governance mechanisms built with Substrate. 

* [Build DAOs with Substrate?](#y)
* [What is Moloch?](#moloch)

## Build DAOs with Substrate <a name = "y"></a>

Substrate is perfect for implementing decentralized autonomous organizations (DAOs)!

The runtime logic is encoded in [Rust](https://www.parity.io/why-rust/) and compiled down to a [WASM binary blob](https://medium.com/polkadot-network/wasm-on-the-blockchain-the-lesser-evil-da8d7c6ef6bd)that is stored on-chain. This architecture facilitates upgrades according to the consensus protocol chosen by developers. Because the consensus logic is included in the runtime, it can also be upgraded (*metagovernance* capabilities). This flexibility allows Substrate DAppchains to evolve and easily incorporate modern research into the runtime logic.

Substrate is admittedly overkill for timestamps, simple token transfers, and other basic blockchain use cases. However, where Substrate really shines is helping developers build high performance mechanisms for decentralized coordination. A good rule of thumb is to **use Substrate when you want to build a community around your mechanism**.

**Upgradability**
* increased flexibility wrt upgrades mitigates the damage of attacks like TheDAO
* also allows the DAO to improve its governance in the future, incorporate privacy-enhancing protocols, etc.

**Shared Security w/o Cost Spillovers**
* interacting with the DAO on Substrate does not carry the risk of incurring high fees when another application/DAO experiences high demand (like how Cryptokitties `=>` state bloat `=>` high fees)
    * this implies a smaller attack surface for economic security (as long as the developer careful with the balance of resources and costs)
* Polkadot will enable deployment in shared security context

**Rust >>> Solidity**
* Rust is **safer** and more well vetted than Solidity; doesn't come with all of the annoying quirks of working with the EVM
    * [Towards a Brighter Future for Smart Contracts]() by Jack Fransham
    * [Why Write Smart Contracts in Rust]() by Jack Fransham
* Rust's language ecosystem is growing independently of Solidity, thereby enabling Substrate projects to benefit from its tooling, libraries, and the network effects of its community

* Some concluding sentence that encourages developers building economic mechanisms to experiment with Substrate (gets all three of these points in a single sentence)
    * Radical Markets specifically (Liberal Radicalism, Quadratic Signalling)

### More Reading
* [A brief summary of everything Substrate and Polkadot](https://www.parity.io/a-brief-summary-of-everything-substrate-polkadot/)
* [What is Parity Substrate](https://www.parity.io/what-is-substrate/) by Jack Fransham
* [Substrate in a nutshell](https://www.parity.io/substrate-in-a-nutshell/)

## What is Moloch? <a name = "moloch"></a>
> *Moloch who entered my soul early! Moloch in whom I am a consciousness without a body! Moloch who frightened me out of my natural ecstasy! Moloch whom I abandon! Wake up in Moloch! Light streaming out of the sky!*

**[Moloch](https://github.com/MolochVentures/moloch)** is minimally viable DAO designed to coordinate funding for the Ethereum ecosystem. Spearheaded by Ameen Soleimani, Arjun Bhuptani, James Young, Layne Haber & Rahul Sethuram, the DAO seeks to overcome the *tragedy of the commons* by incentivizing cohesion among the DAOs members in order to collectively fund open source work.

> *A novel organizational design aiming primarily to more effectively coordinate resources for issuing development grants for Ethereum.* [The Moloch DAO: The Collapsing Firm](https://medium.com/@simondlr/the-moloch-dao-collapsing-the-firm-2a800b3aa2e7), SimonDLR

Members of the Moloch DAO own a native asset referred to as `shares`. Shares are nontransferrable and, in the Substrate implementation, ownership is represented by [maps](https://amarrsingh.github.io/SubstrateCookbook/storage/mapping.html). These shares allow you to do two things:
1. vote on issuing more shares to new members
2. access capital locked up in the DAO by burning shares (`rageQuit`)

To apply for shares, applicants require sponsorship from existing members. The sponsor of an application bonds some capital (0.1 ETH) to submit an application on behalf of aspiring members. Applicants must also offer capital in exchange for a specified number of shares that are requested from issuance.

[`MolochVentures/moloch`](https://github.com/MolochVentures/moloch) is a "Minimally Viable DAO" that follows the protocol outlined above, coded in Solidity, and launched on Ethereum.

MolochDAO is designed to solve the "tragedy of the commons" problem represented by Ethereum's current model for funding open source infrastructure. If successful, this model (or some variation of it) may also be useful to organize funding for other blockchain platforms.

### More Reading
> [Whitepaper](https://github.com/MolochVentures/Whitepaper)

* [Moloch DAO - User Experience Analysis](https://medium.com/@stellarmagnet/moloch-dao-user-experience-analysis-644a0356955) - Yalda Mousavinia (1/20/19)
* [Inside Moloch: A new DAO aims to fix Ethereum](https://decryptmedia.com/5206/fixing-ethereum)
* **[Meditations on Moloch](https://slatestarcodex.com/2014/07/30/meditations-on-moloch/)**