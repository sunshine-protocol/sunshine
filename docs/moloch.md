# What is Moloch?

**[Moloch](https://github.com/MolochVentures/moloch)** is a minimally viable DAO designed to coordinate funding for the Ethereum ecosystem. Spearheaded by Ameen Soleimani, Arjun Bhuptani, James Young, Layne Haber & Rahul Sethuram, the DAO seeks to overcome the *tragedy of the commons* by incentivizing cohesion among the DAOs members in order to collectively fund open source work.

> *A novel organizational design aiming primarily to more effectively coordinate resources for issuing development grants for Ethereum.* [The Moloch DAO: The Collapsing Firm](https://medium.com/@simondlr/the-moloch-dao-collapsing-the-firm-2a800b3aa2e7), SimonDLR

Members of the Moloch DAO own a native asset referred to as `shares`. Shares are nontransferrable and, in the Substrate implementation, ownership is represented by [maps](https://amarrsingh.github.io/SubstrateCookbook/storage/mapping.html). These shares allow you to do two things:
1. vote on issuing more shares to new members
2. access capital locked up in the DAO by burning shares (`rageQuit`)

To apply for shares, applicants require sponsorship from existing members. The sponsor of an application bonds some capital (0.1 ETH) to submit an application on behalf of aspiring members. Applicants must also offer capital in exchange for a specified number of shares that are requested from issuance.

*But how do members fund development?* It's not super obvious, but members can approve a proposal that doesn't lock up additional capital but instead provides proof of some ecosystem contribution. Once approved, the applicant can burn the shares to access a proportional share of the capital locked up by the DAO. The capital withdrawal represents the applicant's grant.

*[back to the README](../README.md)*