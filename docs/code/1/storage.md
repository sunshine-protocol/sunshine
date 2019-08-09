# Storage

We want to establish the following objectives:
1. ease of unit testing to demonstrate best practice
2. minimize calls to storage through optimal assignment of fields to structs -- NOTE instances in which two map calls are necessary and determine if the structs out to be combined `=>` eventually, benchmark this behavior?

## Minimizing Storage Calls

* first I was thinking a lot about minimizing calls to storage so I combined two structs that I had previously been keeping separately
* then, I realized that there were so many struct fields so I considered pairing them in a reasonable way

```rust
pub struct Proposal<AccountId, BlockNumber> {
    // the sponsor's share bond
    sponsor: AccountId,

}
```

You could even just have one struct and maintain the application state as a field. Maybe it might look something like:

```rust
pub struct Proposal<AccountId, BlockNumber< {
    ...
    app_state: State<BlockNumber>,
    ...
}
```

such that 

```rust
pub enum State<T> {
    Applied(T),
    Voting(T),
    Grace(T),
}   
```

The `T` 

## Configuration in Storage

The use of this `config.build` from `srml/balances` is useful for bootstrapping a storage state from no state.