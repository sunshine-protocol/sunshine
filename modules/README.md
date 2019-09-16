# modules

* [committee](./committee), inspired by `council`
* [fund](./fund/), inspired by `treasury`
* [membership](./membership/), inspired by `membership`
* [voting](./voting/), inspired by `elections`

## runtime configuration

sunshine's runtime configuration uses the `voting` module to make different internal decisions:
* `membership` is voted on by the `member`s using their `shares` `=>` the required quorum threshold should be adjustable `=>` additional feature is to make projections on how this will change but this feature has to be added in (see `vote/futarchy/`)
* `committee` is selected by the `member`s using Phragmen with the `member` shares `=>` the committee does not generate any reward but maintains turnout bias for all `fund` decisions (see next decision type)
* `fund` is similar to `treasury`, but provides nuanced governance of the assets held by the organization `=>` a key feature is the parameterization discussed briefly in `monetary-futarchy`

<center><img src="../art/SUNSHINE.png" width="500" height="300"></center>

The `voting` module is designed to be highly configurable to provide nuanced governance over the DAO's `membership`, `committee`, and `fund`. Each of the arrows in the above diagram requires a different configuration of the `voting` module in the runtime. In this sense, the arrows represent governance of a stakeholder set over internal decisions.

## why not just use original srml modules? <a name = "y"></a>

This project's design pulls heavily from patterns in `srml/{membership, council, elections, democracy, staking, treasury`, but doesn't directly use these modules. This decision to not directly import the aforementioned srml modules is explicit and is rooted in the belief that specialization leads to optimization in the context of module development. The SRML as we know it today is optimized for Polkadot's runtime configuration because it was made for the Polkadot relay chain. 

Conversely, **sunshine** comprises of a set of modules that can be included in any parachain and/or parathread runtime to facilitate fund coordination amongst a defined stakeholder set. It's application is not limited to the Polkadot treasury although that is the most immediate use case.

Other features that might be added in the feature include
- [ ] `court` for dispute resolution (to appeal rejected proposals or settle procedural complaints)
- [ ] DAOs as first-class members of `membership` (make `membership`'s governance composable/embeddable?)
- [ ] enable arbitrary DAO splits and merges (*graceful*)
- [ ] structured process for crowdfunding DAOs (and an investment criteria for D2D investment)
- [ ] lending criteria for DAOs (which may also consider membership credit composition)