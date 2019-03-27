# Malkam DAO

**Malkam** is the cheeky cousin of **[Moloch](https://github.com/MolochVentures/moloch)**. At its current stage, Malkam is a sample project to demonstrate DAO development on Substrate. 

* [What is Moloch?](#moloch)
* [Why Build DAOs with Substrate?](#y)

## What is Moloch? <a name = "moloch"></a>
> *Moloch who entered my soul early! Moloch in whom I am a consciousness without a body! Moloch who frightened me out of my natural ecstasy! Moloch whom I abandon! Wake up in Moloch! Light streaming out of the sky!*

**[Moloch](https://github.com/MolochVentures/moloch)** is minimally viable DAO designed to coordinate funding for the Ethereum ecosystem. Spearheaded by Ameen Soleimani, Arjun Bhuptani, James Young, Layne Haber & Rahul Sethuram, the DAO seeks to overcome the *tragedy of the commons* by incentivizing cohesion among the DAOs members in order to collectively fund open source work.

> *A novel organizational design aiming primarily to more effectively coordinate resources for issuing development grants for Ethereum.* [The Moloch DAO: The Collapsing Firm](https://medium.com/@simondlr/the-moloch-dao-collapsing-the-firm-2a800b3aa2e7), SimonDLR

> SHORT SUMMARY OF HOW MOLOCH WORKS (borrow from SIMONDLR and WHITEPAPER?)

### More Reading
> * [`MolochVentures/moloch`](https://github.com/MolochVentures/moloch), [whitepaper](https://github.com/MolochVentures/Whitepaper)

* [The Moloch DAO: The Collapsing Firm](https://medium.com/@simondlr/the-moloch-dao-collapsing-the-firm-2a800b3aa2e7) - simondlr (1/16/19)
* [Moloch DAO - User Experience Analysis](https://medium.com/@stellarmagnet/moloch-dao-user-experience-analysis-644a0356955) - Yalda Mousavinia (1/20/19)
* [Inside Moloch: A new DAO aims to fix Ethereum](https://decryptmedia.com/5206/fixing-ethereum)
* **[Meditations on Moloch](https://slatestarcodex.com/2014/07/30/meditations-on-moloch/)**

## Why Build DAOs with Substrate <a name = "y"></a>
> opening sentence to invite the reader in...

* **use Substrate when you want to build a community around your incentive mechanism**

* Smart contract platforms are generally useful for leveraging composability and network effects (ie interacting with external smart contracts and requiring continuous external calls)
* However, DAOs are pretty much self-contained; they operate independently and do not require external interaction outside of the defined smart contract suite 
* This smart contract suite can actually be abstracted into the module-style organization used to code with Substrate...moreover, it is actually better to do this than to deploy on smart contract platforms

*Upgradability*
* increased flexibility wrt upgrades mitigates the damage of attacks like TheDAO
* also allows the DAO to improve its governance in the future, incorporate privacy-enhancing protocols, etc.
**Siloing Economic Interactions**
* interacting with the DAO on Substrate does not carry the risk of incurring high fees when another application/DAO experiences high demand (like how Cryptokitties `=>` state bloat `=>` high fees)
    * this implies a smaller attack surface for economic security (as long as the developer careful with the balance of resources and costs)
**Rust >> Solidity**
* Rust is safer and more well vetted than Solidity; doesn't come with all of the annoying quirks of working with the EVM
    * [Towards a Brighter Future for Smart Contracts]() by Jack Fransham
    * [Why Write Smart Contracts in Rust]() by Jack Fransham

* Some concluding sentence that encourages developers building economic mechanisms to experiment with Substrate
    * Radical Markets specifically (Liberal Radicalism, Quadratic Signalling)

### More Reading
* [A brief summary of everything Substrate and Polkadot](https://www.parity.io/a-brief-summary-of-everything-substrate-polkadot/)
* [What is Parity Substrate](https://www.parity.io/what-is-substrate/) by Jack Fransham
* [Substrate in a nutshell](https://www.parity.io/substrate-in-a-nutshell/)