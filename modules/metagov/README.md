# Metagovernance

In this folder, there are three different implementations of a fund coordination DAO on Substrate. Each implementation has unique tradeoffs in the context of user behavior. Likewise, the network should assign a **committee** to approve a list of acceptable application architectures. The acceptable architectures should be logically equivalent, but they ought to vary in cost depending on the user behavior. 

> *Change car tires according to the weather. If weather is consistent and not erosive, we can keep the same tires for some time. When it snows or hails, we use the requisite equipment to increase safety.* 

So, the question is **how can I create a governance body that coordinates which application architecture is most favorable in which scenarios?** I also want the mapping between scenarios and architectures to be enforced unless majority of coinvoters vote to change it or 25% of the committee votes to change it. Keeping this at such a low threshold prevents capture by allowing the minority to forcefully switch the algorithm. There must be a quota of switches per time period, but the protocol should automatically switch versions when conditions specify implementation superiority in context. In general, all implementations should be preferred in some scenario -- otherwise, it is by definition regression from some other choice (although wouldn't be that bad to keep back ups).

This demonstrates an interesting application of Substrate's on-chain runtime upgrades in which upgrades may be able to be automatically executed according to some decided upon `conditions => implementation` mapping. This makes the most sense because there is rarely one uniformly superior application architecture. 

The best design depends heavily on the use case in question as well as how stakeholder dynamics change after usage. Moreover, the solution to problems create new problems that are noticed after a lag. With this in mind, it is important to establish best practices for architecture deployment in specific contexts. Eventually I hope that all applications maintain the flexibility to coordinate responsive changes in architecture according to usage patterns. Powered by stakeholder governance, the described system embodies a dynamic organism that optimizes its structure according to its surroundings...like a *Molochameleon* :P

## basic <a href="./basic"></a>

This implementation has a lot of structs for managing state within the runtime storage. Although some of these structs could be merged to decrease the number of storage calls (see *storage* details below), this implementation is easier to read/maintain and is also favorable in instances of application state bloat.

For context, all proposals in Moloch require sponsorship by an existing member. In `sunshine`, direct proposals can be made exclusively by members while outside applications are placed in a queue for a limited amount of time. If the application is not sponsored for voting within the `ApplicationWindow`, then the application is discarded. By separating applications from proposals, this implementation implies less state bloat when there are many applications that are not being sponsored. If the *storage* implementation was chosen in the context of the bloated application queue, there would be much more state bloat given the size of `Proposal`...

## storage <a href="./storage"></a>

This implementation contains all of the required state for what was previously the `Election` and `Application` state, all in the `Proposal` struct.

```rust
/// Generic Moloch Proposal
///
/// request `mint_request` Shares in exchange for `donation` balance staked
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, BlockNumber> {
    // application/proposal/law state
    state: State<BlockNumber>,
    // the proposal sponsor must be a member
    sponsor: Option<AccountId>,
    // the sponsor's share bond (also vote in support by default)
    sponsor_bond: Option<Shares>,
    // the applicant (None if direct proposal by existing member)
    applicant: Option<AccountId>,
    // donation to the DAO (None if grant proposal)
    donation: Option<Balance>,
    // mint request for membership shares (None if pure donation)
    mint_request: Option<Shares>,
    // threshold for passage
    threshold: Shares,                      // TODO: abstract into a voting module
    // shares in favor, shares against
    scoreboard: (Shares, Shares),           // TODO: abstract into a voting module
    // supporting voters (voted yes)
    ayes: Vec<(AccountId, Shares)>,
    // against voters (voted no)
    nays: Vec<(AccountId, Shares)>,         // TODO: if enough no votes, then bond slashed `=>` otherwise returned
}
```

Interestingly, the state field uses an enum for tracking the proposal's *state*:

```rust
/// Generic enum for representing proposal state
///
/// -- the type indicates when the state will end
pub enum State<T> {
    // application period has commenced
    Application(T),
    // voting period has commenced
    Proposal(T),
    // the grace period has commenced
    Law(T),
}
```

This implementation minimizes calls to runtime storage, but is much harder to read/debug. The use of the enum to track `Proposal` state is elegant, but I feel like there is more to associate with each phase than just a block number for holding the expiration time. How can this be done better?

## readable <a href="./readable"></a>

This is my favorite of the three. It doesn't make as many calls to runtime storage as *basic* and also doesn't suffer from state bloat in the context of a full (unsponsored) application queue. I'll be continuing development with this implementation.

<!-- ## Loosely Coupled Modules (in progress)

* how to use multiple files in Rust in general
* how to build multiple interacting modules

## DAOception (in progress)

* how to use `EnsureOrigin` to implement *DAOs in DAOs*

## Instancing DAOs (in progress)

* running multiple variations of DAOs at the same time
* subDAOs for delegation and other common organizational patterns (besides pure hierarchy) -->