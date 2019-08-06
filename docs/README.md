# Documentation

Sunshine is a fund coordination decentralized autonomous organization (DAO) built on Substrate. The current iteration demonstrates the interactive patterns common to most DAOs in the context of Substrate's runtime. This project was originally inspired by [Moloch](https://github.com/moloch).

## Motivation

A common mistake is to start designing user-facing applications by brainstorming a layout. When this is done for blockchain-based applications, the UI looks a lot like centralized, web-based application because the designer inadvertently maps to the layouts that are most familiar. While this might seem preferrable, it ignores the fact that blockchain-powered applications offer an entirely new set of features, many of which we have yet to invent/discover. Indeed, UI and layout should be extracted from the feature set and not the other way around. Only through closely examining the relative features of modern blockchains can we extract the best UX for applications.

An example on Ethereum is the growing popularity of meta-transactions. While meta-transactions are quite clever, they rely on centralized relayers on L2 to submit transaction batches. This sacrifices security and is limited in terms of long-term sustainability. The incentivization of decentralized relayer markets is still an open problem (I expect this to be solved by configurable block rewards on Substrate that can be used to fund relayer networks and other auxiliary support).

## Purpose

**Document Mechanism Design Choices for Maximal Forkability**

The **main objective** of this project is to clearly document the often ignored design decisions made that other projects can make different choices when appropriate for the context of their use case. Given Substrate's modularity, there isn't always a *best* way to do something, often times a few choices provide varying associated tradeoffs.

The goal of [the talk at Sub0](https://www.youtube.com/watch?v=eguDIG11nW8) was to convey the significant complexity of designing a minimal fund coordination DAO. The talk covered the clash between lock-in and instant withdrawals before briefly introducing a closed loop for bonding proposals and incentivizing member action. Thereafter, a slide read: `There is a lot of hidden complexity` with the following list of open questions:
* Should the voting period transition to the grace period immediately once a threshold of support is achieved?
* Is the voting period time window finite and, if so, what is to be done with stale proposals?
* Is the proposal immediately executed once it passes a threshold in the voting period or should the execution delay match the *grace period* during which dissenters can exit?
* Is there an appeals process to reverse voting outcomes? *dispure resolution court*
* Do we require an additional actor to process the proposal if is approved? Incentives?
* What is the incentive to vote earlier? What is the incentive to vote at all?
* How are votes weighted (or *preferences aggregated*)? Does voter turnout influence the required passing thresholds?
* How do we mitigate bribery, coercion, and collusion?

To be honest, this doesn't cover the half of it -- we might also consider how to collateralize exit requests for faster exits, the mental overhead of introducing a secondary token, or even the delegation of shares in the voting process according to proposal *types*.

Most of these ideas might just add unncessary complexity and limit accessibility, but some might actually work and, moreover, we might figure out a way to engage user interaction and overcome any initial barriers. Indeed, I am hopeful that we can build effective and accessible coordination mechanisms with Substrate.

To this end, I think it's increasingly important to pursue *fast* experimentation in parallel (like *parachains* on **Polkadot** 🚀). It is necessary to experiment with diverse uses cases and varying parameterizations in order to discover the always-changing optimal structures.

Indeed, these economic mechanisms will never be *finished* because of their interaction with a changing world -- upgradability is necessary! With Substrate's runtime flexibility, we can pursue an eternal ideal that balances accessibility with optimized incentive design.

*Current [Research Reading List](./library.md)*
