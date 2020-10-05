## Bounty Pallet

This pallet placed 3rd 🏆 in [Hackusama](https://hackusama.devpost.com/submissions) with a demo that used a custom substrate-subxt client to update github issue information based on changes to chain state.

### Post Bounties

Anyone can post bounties as long as the amount is above the module minimum. The module minimum is set in the pallet's `Trait`.

```rust
pub trait Trait {
    ...
    /// Minimum deposit to post bounty
    type MinDeposit: Get<BalanceOf<Self>>;
}
```

The public runtime method signature is

```rust
fn post_bounty(
    origin,
    issue: EncodedIssue,
    info: T::IpfsReference,
    amount: BalanceOf<T>,
) -> DispatchResult
```

The `amount` is checked against the module constraints

### Contribute to Bounties

Anyone can contribute to bounties. There are no refunds and there is no representation in spending governance (*[y](../democracy.md)*). The only constraint is that outside contributions must exceed the relevant module constant.

```rust
pub trait Trait {
    ...
    /// Minimum contribution to posted bounty
    type MinContribution: Get<BalanceOf<Self>>;
}
```

### Apply for Bounty

Anyone except the poster can apply for a bounty.

### Approve Bounty

Only the account that posted the bounty can approve submissions. Submission approval immediately transfers funds to the recipient.

### Next Steps

This module works for single account governance, but isn't sufficiently expressive for democracy (direct and representative). Future versions will allow contributors to select representatives and vote to approve submissions. See the `grant` pallet for an example of an on-chain grants program that uses org voting to make grant decisions.