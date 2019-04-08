# MoloChameleon DAO 🐸☕️

The DAO logic is implemented in [`runtime/src/dao.rs`](./runtime/src/dao.rs). At the moment, the implementation is undergoing heavy refactoring in response to testing.

There are a few issues to encourage outside contribution, but it might be better to wait until testing is finished (=> refactoring for v1 is complete).

## Current Priorities

* Refactoring based on what I learn from `srml/staking`
* Testing
* Adding Documentation and Implementation FAQs
    * focus on the share abstraction, the proposal process, and the long-term vision

### Ongoing Thoughts

One obvious question would be: **isn't it redundant to store both the members as well as pending proposals in the `Pool` struct? Well, I actually have a vision for this project beyond its current capabilities in which an arbitrary number of Pools can be instantiated with arbitrary number of members. 

Because complexity blows up when this decision is made, I'm going to table it for now. I think it'd be especially cool if `Pool`s took the shape of CRDTs, thereby allowing for arbitrary merges and forks in which the membership set changes according to these changes at the `Pool` level. Like I said, too much complexity for a tutorial. I'm thinking I'll probably call that implementation Sunshine DAO.

To orient the project towards this vision, I will shift some of the pressure from the storage and onto the `Pool` struct though. I think this is for the best and will benefit the modularity of the final implementation.

#### Dynamic Thresholds

* obviously the current threshold is quite weak; we're just checking if greater than 50% of the votes agree, not accounting for lack of participation

#### Nomination
* allow donations that support members of the DAO (like charities)
    * would be cool if donations followed Liberal Radicalism somehow; basically we redistribute funds based on continuous elections for the current members; *frogman* might be relevant, but I don't want to use it here unnecessarily

#### Async RageQuit

Any protocol that has a RageQuit function ought to implement this function such that it returns a `future` and then we can do stuff with the funds that will be returned from the RageQuit before the function even returns. What might we want to do? Maybe use the rageQuit funds as collateral for something else?