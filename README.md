# SunshineDAO

| [Build DAOs with Substrate](#y) | [Introduction to Moloch](#moloch) | [More Reading](./docs/library.md) 
| ------------- | ------------- | ------------- |

**Sunshine** is a fund coordination DAO on Substrate inspired by **[Moloch](https://github.com/MolochVentures/moloch)**. The basic idea is to use Moloch's governance mechanism to coordinate membership and raise capital. Thereafter, signalling will strive to be generic and flexible, but I am first prioritizing functionality before abstracting shared behavior. I've identified the significant tasks 

*Necessary*
- [ ] signalling
    - [ ] members vote (weighted by stake) to fund grants (applications with only the transaction fee)
    - [ ] members vote to accept new members
- [ ] managing sets
    - [ ] changing sets
    - [ ] tracking groups with `Origin` and maps in `decl_storage`
- [ ] lock-in vs fast withdrawal (voting periods) <=> exit mechanisms
- [ ] design criteria

*Optional*
- [ ] (optional/extension) members vote to grant voice in the DAO based on community activism could be useful, but it opens an attack in which the malicious entity increases their relative vote advantage by overpowering the group and voting in someone that agrees with their side on a prominent issue
- [ ] (optional/extension) members vote to reject new members
- [ ] (optional/extension) using a prediction on an oracle to lock-in a vote on an outcome
- [ ] (optional/extension) creating SubDAOs with a strict subset of the members of the DAO, but the rest of the members weight all their support on someone in the SubDAO


## What is Moloch? <a name = "moloch"></a>

**[Moloch](https://github.com/MolochVentures/moloch)** is a minimally viable DAO designed to coordinate funding for the Ethereum ecosystem. Spearheaded by Ameen Soleimani, Arjun Bhuptani, James Young, Layne Haber & Rahul Sethuram, the DAO seeks to overcome the *tragedy of the commons* by incentivizing cohesion among the DAOs members in order to collectively fund open source work.

> *A novel organizational design aiming primarily to more effectively coordinate resources for issuing development grants for Ethereum.* [The Moloch DAO: The Collapsing Firm](https://medium.com/@simondlr/the-moloch-dao-collapsing-the-firm-2a800b3aa2e7), SimonDLR

Members of the Moloch DAO own a native asset referred to as `shares`. Shares are nontransferrable and, in the Substrate implementation, ownership is represented by [maps](https://amarrsingh.github.io/SubstrateCookbook/storage/mapping.html). These shares allow you to do two things:
1. vote on issuing more shares to new members
2. access capital locked up in the DAO by burning shares (`rageQuit`)

To apply for shares, applicants require sponsorship from existing members. The sponsor of an application bonds some capital (0.1 ETH) to submit an application on behalf of aspiring members. Applicants must also offer capital in exchange for a specified number of shares that are requested from issuance.

*But how do members fund development?* It's not super obvious, but members can approve a proposal that doesn't lock up additional capital but instead provides proof of some ecosystem contribution. Once approved, the applicant can burn the shares to access a proportional share of the capital locked up by the DAO. The capital withdrawal represents the applicant's grant.

**[MORE READING](./docs/library.md)**

## Build DAOs with Substrate <a name = "y"></a>

Substrate is great for implementing decentralized autonomous organizations (DAOs).

When a developer builds with Substrate, the blockchain's runtime logic is encoded in [Rust](https://www.parity.io/why-rust/) and compiled down to a [WASM binary blob](https://medium.com/polkadot-network/wasm-on-the-blockchain-the-lesser-evil-da8d7c6ef6bd) that is stored on-chain. This architecture facilitates upgrades according to the consensus protocol chosen by developers. Because the consensus logic is included in the runtime, it can also be upgraded (=> capacity for *metagovernance*). This flexibility allows Substrate DAppchains to evolve and easily incorporate modern research into the runtime logic.

Substrate may be overkill for timestamping data, executing simple token transfers, and realizing other basic blockchain use cases. Indeed, *it doesn't make sense to take a jet for a 10 km commute.*

However, where Substrate really shines is helping developers build high performance mechanisms for decentralized coordination. A good rule of thumb is to **use Substrate when you want to build a community around your mechanism**.

**Upgradability**
* enables future improvement of governance i.e. incorporate privacy-enhancing protocols like ZK stuff
* mitigates the damage of attacks like [*The DAO*](https://medium.com/swlh/the-story-of-the-dao-its-history-and-consequences-71e6a8a551ee) by facilitating dynamic response strategies via metagovernance

**Rust >>> Solidity**
* Rust is safer and more well vetted than Solidity; it doesn't come with all of the annoying quirks of working with the EVM
    * [Towards a Brighter Future for Smart Contracts](http://troubles.md/posts/rust-smart-contracts/) by Jack Fransham
    * [Why Write Smart Contracts in Rust](http://troubles.md/posts/why-write-smart-contracts-in-rust/) by Jack Fransham, Parity
    * [Why Rust](https://www.parity.io/why-rust/) by Parity
* Rust's language ecosystem is growing independently of Solidity, thereby enabling Substrate projects to benefit from its tooling, libraries, and the network effects of its community
* **Rust -> WASM >>> Solidity -> EVM**
    * [WASM on the blockchain, the lesser evil](https://medium.com/polkadot-network/wasm-on-the-blockchain-the-lesser-evil-da8d7c6ef6bd) by Jack Fransham, Parity

**Shared Security w/o Cost Spillovers**
* [Polkadot](https://medium.com/polkadot-network/polkadot-the-foundation-of-a-new-internet-e8800ec81c7) will foster deployment in a shared security context
* interacting with DAOs built on Substrate does not carry the risk of incurring high fees when another application/DAO experiences high demand (like how Cryptokitties `=>` state bloat on Ethereum `=>` high fees)
    * this implies a smaller attack surface for economic security (as long as the developer is careful with the balance of resources and costs)

Substrate's value proposition arises from the composition of these benefits. 
1. Compilation from Rust to WASM facilitates on-chain upgrades, thereby increasing the application's relative flexibility. 
2. Rust's low-level handling encourages creative code patterns that optimize performance while protecting memory safety. 
3. Deployment in the context of Polkadot fosters shared security without expensive spillover costs from other parachain activity.

> Truth is that we haven't even scratched the surface of the novel applications that can be built with `(1) + (2) + (3)`

As an example, consider the proposal process implemented by the [Ethereum-based MolochDAO implementation](https://github.com/MolochVentures/moloch). This implementation is probably close to optimized in the context of Solidity, but it actually suffers from head-of-line blocking for proposals. More specifically, a proposal must wait for a previous proposal to be processed before it can be approved. Moreover, there are defined limits on how many proposals can be processed in a given time period. An optimized implementation of the proposal process probably looks something like [Rhododendron](https://github.com/paritytech/rhododendron). By leveraging asynchronous primitives in Rust, we can increase the throughput capacity of the proposal process. More details will be shared soon, but this optimization is pretty high on *my priority queue*.

### *Radical* Substrate

Hopefully I've convinced you that Substrate is worth checking out. Personally, I'm very excited to help realize robust implementation of mechanisms inspired by [Radical Markets](http://radicalmarkets.com/) and the other fascinating projects shaping Cryptoeconomics as a discipline.

Specifically, I think it would be cool to build
* Liberal Radicalism DAO
* Quadratic Signalling Mechanisms w/ Authenticated Polling
* Income Share Agreements
* Auction Models

If you're still not convinced and would like more details, feel free to check out the articles listed below, [the official documentation](https://docs.substrate.dev/), and the [github](https://github.com/paritytech/substrate/).
* [A brief summary of everything Substrate and Polkadot](https://www.parity.io/a-brief-summary-of-everything-substrate-polkadot/)
* [What is Parity Substrate](https://www.parity.io/what-is-substrate/)
* [Substrate in a nutshell](https://www.parity.io/substrate-in-a-nutshell/)