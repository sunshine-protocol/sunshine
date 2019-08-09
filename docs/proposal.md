# New Proposal Architecture is Interesting

started with something like this when I was considering types:

```rust
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Application<AccountId, Balance, BlockNumber> {
    // requested recipient of shares
    applicant: AccountId,
    // the applicant's donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,
    // BlockNumber at initial application
    start: BlockNumber,
}

/// Spending Proposal
///
/// separate fields into multiple structs (create Election struct?)
/// `=>` would need to link Election to every Proposal
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Proposal<AccountId, Balance> {
    // the proposal sponsor must be a member
    sponsor: AccountId,
    // the sponsor's vote (/bond in support, like a voter)
    sponsor_vote: Shares,
    // the applicant does not need to be a member
    applicant: Option<AccountId>,
    // applicant's donation to the DAO
    donation: Option<Balance>,
    // membership shares requested
    shares_requested: Shares,
}

/// Election State
///
/// TODO: (1) add shares_requested field
///        (2) abstract out voting and replace threshold with `VoteThreshold` for more algorithms
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Election<AccountId, BlockNumber> {
    // applicant accountId (to prevent when == &voter)
    app_sender: AccountId,
    // BlockNumber at initial proposal
    vote_start: BlockNumber, // TODO: eventually consider adding GraceWindow and VoteWindow; unique to every proposal (~complexity for flexibility)
    // initiated by proposal passage in `vote`
    grace_end: Option<BlockNumber>,
    // threshold for passage
    threshold: Shares, // TODO: update based on dilution dynamics in `vote` (like in MolochDAO); calculate_threshold based on donation/shares ratio relative to average
    // shares in favor, shares against
    state: (Shares, Shares),
    // supporting voters (voted yes)
    ayes: Vec<(AccountId, Shares)>,
    // against voters (voted no)
    nays: Vec<(AccountId, Shares)>, // TODO: if enough no votes, then bond slashed `=>` otherwise returned
}

/// Voter state (for lock-in + 'instant' withdrawals)
#[derive(Encode, Decode, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Member {
    // total shares owned by the member
    all_shares: Shares,
    // shares reserved based on pending proposal support
    reserved_shares: Shares,
    // shares reserved for voters
    voter_bond: Option<Shares>,
    // TODO: consider more stateful pattern via `BTreeMap<Hash, Shares>` for vote pairs
}
```

## Then I Optimized to Limit Multiple Storage Calls

* not even really sure if there are incredibly tangible costs of multiple storage costs but I think this is just hygenic refactoring
* if I wanted to make the proposal less stateful, I could **rename** this struct **election** because it is very stateful, but I would just have to disable voting on it based on `.state`

I basically separate `state` into three categories: 
* application `=>` proposal is much more limited
* proposal `=>` voting has commenced and the proposal is under consideration
* law `=>` the grace period follows the inner type here for `GraceWindow`

Note there is some consideration being given to vary the `GraceWindow` and `VoteWindow` based on the tutorial -- in which case, maybe we would make 

```rust
pub enum State<T> {
    Application(T, T),
    Proposal(T, T),
    Law(T, T),
}
```
such that the the type is `BlockNumber`, but I just 


### Covering All Application Variations

* application

* sponsored proposal

* direct proposal with self as recipient

* direct proposal with other account as recipient (maybe should require approval from receiving entity)

> **need to consider how the DAO can sign messages and make method calls** `=>` is there a way to dispatch function calls from the DAO member set? Yes, I think the way is to use the special origin. Implement next...this is the path towards the functionality of DAOs within DAOs `=>` the requirement is that DAOs can sign messages confirming themselves as the recipients of funds