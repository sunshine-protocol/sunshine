# Documentation

Sunshine is a fund coordination decentralized autonomous organization (DAO) built on Substrate. The current iteration demonstrates the interactive patterns common to most DAOs in the context of Substrate's runtime. 

Even so, this implementation is far from optimized both in terms of performance as well mechanism design. As I *level-up* in Rust, I will keep integrating the patterns I learn into the existing implementation. In my free time, I'll also continue familiarizing myself with the deluge of fresh mechanism ideas circulating the space (e.g. cryptotwitter ™️). 

## Document Mechanism Design Choices and Thought Process

My **main objective** with these docs is to clearly document the choices I make so that other projects can make different choices when appropriate for the context of their use case. [MolochDAO](https://github.com/MolochVentures/moloch) encourages developers to `STEAL THIS CODE` -- this is a clear step in the right direction.

The goal of my talk at [Sub0](https://sub0.parity.io/) was to convey the significant complexity of designing a minimal fund coordination DAO. In the talk, I covered the clash between lock-in and instant withdrawals before briefly introducing a closed loop for bonding proposals and incentivizing member action. Thereafter, I had a slide that read: `There is a lot of hidden complexity` with the following list of open questions:
* Should the voting period transition to the grace period immediately once a threshold of support is achieved?
* Is the voting period time window finite and, if so, what do we want to do with stale proposals?
* Is the proposal immediately executed once it passes a threshold in the voting period or should the execution delay match the *grace period* during which dissenters can exit?
* Is there an appeals process to reverse voting outcomes?
* Do we require an additional actor to process the proposal if is approved? Incentives?
* What is the incentive to vote earlier? What is the incentive to vote at all?
* How are votes weighted (or *preferences aggregated*)? Does voter turnout influence the required passing thresholds?
* How do we mitigate bribery, coercion, and collusion?

To be honest, this doesn't cover the half of it -- we might also consider how to collateralize exit requests for faster exits, the mental overhead of introducing a secondary token, or even the delegation of shares in the voting process according to proposal *types*.

Most of these ideas might just add unncessary complexity and limit accessibility, but some might actually work and, moreover, we might figure out a way to engage user interaction and overcome any initial barriers. Indeed, I am hopeful that we can build effective and accessible coordination mechanisms with Substrate.

To this end, I think it's increasingly important to pursue *fast* experimentation in parallel (like *parachains* on **Polkadot** 🚀). It is necessary to experiment with diverse uses cases and varying parameterizations in order to discover the always-changing optimal structures.

Indeed, these economic mechanisms will never be *finished* because of their interaction with a changing world -- upgradability is necessary! With Substrate's runtime flexibility, we can pursue an eternal ideal that balances accessibility with optimized incentive design.

*Current [Research Reading List](./library.md)*