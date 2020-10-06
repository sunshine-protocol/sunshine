## Org Pallet

This pallet handles organization membership and governance. Each weighted group of accounts stored in this pallet has a unique `OrgId`. This identifier is often used in inheriting modules to establish ownership of the organization over associated state.

### Share Ownership

Each member (`AccountId`) in an org has some quantity of `Shares` in proportion to their relative ownership and voting power. This ownership metadata is stored in runtime storage like

```rust, ignore
double_map OrgId, AccountId => Option<ShareProfile<T>>;
```

Pallets that inherit this pallet can check membership of an `AccountId` in an `OrgId` by checking if the map associated with the key: `OrgId, AccountId` is `Some(ShareProfile<T>)`. There is an associated method for this purpose.

```rust, ignore
let auth = <org::Module<T>>::is_member_of_group(org, &who);
ensure!(auth, Error::<T>::NotAuthorized);
```

### Default Governance

Every group has a sudo `Option<AccountId>`. This position is set in the organization state upon initialization.

```rust, ignore
pub struct Organization<AccountId, OrgId, IpfsRef> {
    /// Optional sudo, encouraged to be None
    sudo: Option<AccountId>,
    /// Organization identifier
    id: OrgId,
    /// The constitution
    constitution: IpfsRef,
}
```

The sudo is intended to be a representative selected by the group to _keep things moving_, but their selection will be easily revocable. The `rank` module expresses representative selection with enforced term limits for this exact purpose.