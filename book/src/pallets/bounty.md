## Bounty Pallet

This pallet placed 3rd 🏆 in [Hackusama](https://hackusama.devpost.com/submissions) with a submission that included a custom substrate-subxt client to update github issue information based on changes to chain state.

### Post Bounties

Anyone can post bounties as long as the amount is above the module minimum. The module minimum is set in the pallet's `Trait`.

```rust, ignore
pub trait Trait {
    ...
    /// Minimum deposit to post bounty
    type MinDeposit: Get<BalanceOf<Self>>;
}
```

The public runtime method signature is

```rust, ignore
fn post_bounty(
    origin,
    issue: EncodedIssue,
    info: T::IpfsReference,
    amount: BalanceOf<T>,
) -> DispatchResult
```

The `amount` is checked against the module constraints. The `issue` input is the binary encoding of github issue metadata. 

```rust, ignore
type EncodedIssue = Vec<u8>;
```

The storage in this pallet uses a map's keyset to enforce a limit of one github issue per posted bounty.


```rust, ignore
decl_storage!{
    /// Prevent overlapping usage of issues
    pub IssueHashSet get(fn issue_hash_set): map
        hasher(blake2_128_concat) EncodedIssue => Option<()>;
}
```

The first line in this method checks that the encoded issue metadata has not been associated with an another bounty on-chain.

```rust, ignore
ensure!(<IssueHashSet>::get(issue.clone()).is_none(), Error::<T>::IssueAlreadyClaimedForBountyOrSubmission);
```

This global hashset pattern is useful when defining a 1-to-1 mapping between an off-chain identity (e.g. unique github issue) and an on-chain object (e.g. bounty).

### Contribute to Bounties

Anyone can contribute to bounties. There are no refunds and there is no representation in spending governance. The only constraint is that outside contributions must exceed the module constant.

```rust, ignore
pub trait Trait {
    ...
    /// Minimum contribution to posted bounty
    type MinContribution: Get<BalanceOf<Self>>;
}
```

The public runtime method signature is

```rust, ignore
fn contribute_to_bounty(
    origin,
    bounty_id: T::BountyId,
    amount: BalanceOf<T>,
) -> DispatchResult
```

The first line checks the amount exceeds the module constant.

```rust, ignore
ensure!(amount >= T::MinContribution::get(), Error::<T>::ContributionMustExceedModuleMin);
```

### Apply for Bounty

Anyone except the poster can apply for a bounty. The issue associated with the application (*submission*) must be unique and independent from the bounty issue to which it is applying. Likewise, the bounty identifier that the submission references must exist in on-chain storage in order for the submission to be valid.

Here is the runtime method header with the checks required for valid submissions.

```rust, ignore
fn submit_for_bounty(
    origin,
    bounty_id: T::BountyId,
    issue: EncodedIssue,
    submission_ref: T::IpfsReference,
    amount: BalanceOf<T>,
) -> DispatchResult {
    ensure!(<IssueHashSet>::get(issue.clone()).is_none(), Error::<T>::IssueAlreadyClaimedForBountyOrSubmission);
    let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
    let submitter = ensure_signed(origin)?;
    ensure!(submitter != bounty.depositer(), Error::<T>::DepositerCannotSubmitForBounty);
    ensure!(amount <= bounty.total(), Error::<T>::BountySubmissionExceedsTotalAvailableFunding);
    ...
}
```

If any of these checks fail, the method is still safe because no storage values have been changed. This is demonstrates the *verify first, push to storage last* principle.

### Approve Bounty

Only the account that posted the bounty can approve submissions. Submission approval immediately transfers funds to the recipient.

Here is the runtime method header with the checks required for valid submissions.

```rust, ignore
fn approve_bounty_submission(
origin,
submission_id: T::SubmissionId,
) -> DispatchResult {
    let approver = ensure_signed(origin)?;
    let submission = <Submissions<T>>::get(submission_id).ok_or(Error::<T>::SubmissionDNE)?;
    ensure!(submission.state().awaiting_review(), Error::<T>::SubmissionNotInValidStateToApprove);
    let bounty_id = submission.bounty_id();
    let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
    ensure!(bounty.total() >= submission.amount(), Error::<T>::CannotApproveSubmissionIfAmountExceedsTotalAvailable);
    ensure!(bounty.depositer() == approver, Error::<T>::NotAuthorizedToApproveBountySubmissions);
    // execute payment
    T::Currency::transfer(
        &Self::bounty_account_id(bounty_id),
        &submission.submitter(),
        submission.amount(),
        ExistenceRequirement::KeepAlive,
    )?;
    ...
```

### Next Steps

This module works for single account governance, but isn't sufficiently expressive for democracy (direct and representative). Future versions will allow contributors to select representatives and vote to approve submissions. See the `grant` pallet for an example of an on-chain grants program that uses org voting to make grant decisions.