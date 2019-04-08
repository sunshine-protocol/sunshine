# MoloChameleon DAO 🐸☕️

The DAO logic is implemented in [`runtime/src/dao.rs`](./runtime/src/dao.rs). At the moment, the implementation is undergoing heavy refactoring in response to testing.

There are a few issues to encourage outside contribution, but it might be better to wait until testing is finished (=> refactoring for v1 is complete).

## Current Priorities

* Refactoring based on what I learn from `srml/staking`
* Testing
* Adding Documentation and Implementation FAQs
    * focus on the share abstraction, the proposal process, and the long-term vision

### Ongoing Thoughts

Since the infamous DAO hack, Dao implementations as well off-chain scaling mechanisms (ie Plasma) have prioritized instant withdrawals (and for good reason). At the same time, if a member of the DAO sponsors or votes in support of a proposal, they ought to be locked into any resulting governance changes at least long enough for those that disagreed to leave the DAO. 

However, this process breaks down if we don't limit processing to one proposal at a time. This is clearly shown with an example:

Let's assume that we have proposals A, B, and C such that A was proposed first, B was proposed second, and C was proposed third. If a voter votes against A, but in support of B, he will not be able to exit until after B is passed, thereby sacrificing his right to a fast exit once A is passed. Any protocol that strives to increase proposal throughput will run into this fundamental problem.

One *solution* is to split the DAO up into subDAOs based on the topic area. Every week, the voters of the larger DAO could vote on a budget to allocate to each subDAO (if they disagree with the budget, they can leave). Each subDAO could then allocate up to their budget to incoming proposals. Moreover, this allows proposals to be processed in parallel. It is kind of hacky, which is why I haven't committed to pursuing it.

Indeed, adding this subDAO really blows up the complexity of the sample and adds a lot of new decisions that have to be made with respect to the governance structure of such a system. Therefore, I'm going to table it for now. 

I think it'd be especially cool if `Pool`s took the shape of CRDTs, thereby allowing for arbitrary merges and forks in which the membership set changes according to these changes at the `Pool` level. Like I said, too much complexity for a tutorial. I'm thinking I'll probably call that implementation Sunshine DAO.

To orient the project towards this vision, I will shift some of the pressure from the storage and onto the `Pool` struct (in the Sunshine DAO fork). I think this is for the best and will benefit the modularity of the final implementation.

#### Dynamic Thresholds

* obviously the current threshold is quite weak; we're just checking if greater than 50% of the votes agree, not accounting for lack of participation

#### Async RageQuit

Any protocol that has a RageQuit function ought to implement this function such that it returns a `future` and then we can do stuff with the funds that will be returned from the RageQuit before the function even returns. What might we want to do? Maybe use the rageQuit funds as collateral for something else? This would better fulfill the instant withdrawal criterion.