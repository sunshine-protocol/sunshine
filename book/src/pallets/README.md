# Pallets

Runtime logic is expressed in Rust libraries, colloquially referred to as Pallets. While the most important design criterion is readability, developers are generally encouraged to minimize on-chain storage and computation.

All objects and relationships are in [`./utils`](https://github.com/sunshine-protocol/sunshine-bounty/tree/master/utils). Module implementations that conform to the Substrate pallet rules are in [`./pallets/*`](https://github.com/sunshine-protocol/sunshine-bounty/tree/master/pallets).

## Dashboard

**Stable**
* bank
* bounty
* donate
* drip
* org
* treasury
* vote
* vote-direct

**In-Progress**
* rank
* bounty2
* court
* grant
* insurance
* kickback
* moloch
* recovery
* rfp

## Org

Every organization encodes membership with ownership expressed as `Vec<(AccountId, Shares)>`. Each org has an `OrgId`, which is used to establish ownership of state associated with the group.

## Vote

Provides functionality for dispatching votes with the given group's membership as the electorate. These votes may be weighted by the group's ownership or 1 account 1 vote.

## Court

Uses `vote` to dispatch votes to resolve disputes between two parties. Like insurance, one party might only agree to enter into an external contract with the other party if they agree to stake collateral and forfeit that collateral in the event that the dispatched vote resolves against them.

## Donate

Allows any `AccountId` to transfer funds to an `OrgId` such that the funds are distributed to the members of the group in proportion to their ownership in the group.

## Bank

Enables orgs to create joint bank accounts with spends governed by group votes.

## Bounty

Allows individual accounts to post bounties and govern/supervise execution. Supports outside contributions.