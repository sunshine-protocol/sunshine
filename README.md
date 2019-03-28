# Malkam DAO

**Malkam** is the cheeky cousin of **[Moloch](https://github.com/MolochVentures/moloch)**. At its current stage, Malkam is a sample project to demonstrate DAO development on Substrate. 

* [Build DAOs with Substrate?](#y)
* [What is Moloch?](#moloch)

## Build DAOs with Substrate <a name = "y"></a>

Substrate is marketed as the *blockchain framework for innovators*. But what can or should you build with Substrate? Here, I'll make the argument that you should **build with Substrate when you want to build a community around your decentralized application/mechanism**.

The [Cryptokitties](https://github.com/shawntabrizi/substrate-collectables-workshop) and [TCR](https://github.com/parity-samples/substrate-tcr) tutorials helped us become acclimated to development in the context of the Substrate Runtime Module Library. By demonstrating useful patterns in practice, these tutorials help developers struggling to **overcome the steep learning curve that comes with coding in Rust**.

Still, some people may not be convinced that overcoming this learning curve is worth it. It is true 

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