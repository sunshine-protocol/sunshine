# runtime

* [membership](#membership)
* [committee](#committee)
* [fund](#fund)
* [voting](#voting)

## membership <a name = "membership"></a>

*moloch with metagovernance*

```rust
impl membership::Trait<membership::Instance1> for Runtime {
	type Event = Event;
	type AddOrigin = committee::EnsureProportionMoreThan<_1, _2, AccountId, CommitteeCollective>;
	type RemoveOrigin = committee::EnsureProportionMoreThan<_1, _2, AccountId, CommitteeCollective>;
	type SwapOrigin = committee::EnsureProportionMoreThan<_1, _2, AccountId, CommitteeCollective>;
	type ResetOrigin = committee::EnsureProportionMoreThan<_1, _2, AccountId, CommitteeCollective>;
	type MembershipInitialized = FundCommittee;
	type MembershipChanged = FundCommittee;
}
```

## committee <a name = "committee"></a>

```rust
type CommitteeCollective = committee::Instance1;
impl committee::Trait<CommitteeCollective> for Runtime {
    type Origin = Origin;
    type Proposal = Call;
    type Event = Event;
}
```

## fund <a name = "fund"></a>

*treasury with metagovernance*

```rust
parameter_types! {
	pub const ProposalBond: Permill = Permill::from_percent(5);
	pub const ProposalBondMinimum: Balance = 1 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 1 * DAYS;
	pub const Burn: Permill = Permill::from_percent(50);
}

impl treasury::Trait for Runtime {
	type Currency = Balances;
	type ApproveOrigin = collective::EnsureMembers<_4, AccountId, CommitteeCollective>;
	type RejectOrigin = collective::EnsureMembers<_2, AccountId, CommitteeCollective>;
	type Event = Event;
	type MintedForSpending = ();
	type ProposalRejection = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
}
```

## voting <a name = "voting"></a>

```rust
parameter_types! {
	pub const CandidacyBond: Balance = 10 * DOLLARS;
	pub const VotingBond: Balance = 1 * DOLLARS;
	pub const VotingFee: Balance = 2 * DOLLARS;
	pub const MinimumVotingLock: Balance = 1 * DOLLARS;
	pub const PresentSlashPerVoter: Balance = 1 * CENTS;
	pub const CarryCount: u32 = 6;
	// one additional vote should go by before an inactive voter can be reaped.
	pub const InactiveGracePeriod: VoteIndex = 1;
	pub const ElectionsVotingPeriod: BlockNumber = 2 * DAYS;
	pub const DecayRatio: u32 = 0;
}

impl voting::Trait for Runtime {
	type Event = Event;
	type Currency = Balances;
	type ChangeMembers = Committee;
	type CandidacyBond = CandidacyBond;
	type VotingBond = VotingBond;
	type VotingFee = VotingFee;
	type MinimumVotingLock = MinimumVotingLock;
	type PresentSlashPerVoter = PresentSlashPerVoter;
	type CarryCount = CarryCount;
	type InactiveGracePeriod = InactiveGracePeriod;
	type VotingPeriod = ElectionsVotingPeriod;
	type DecayRatio = DecayRatio;
}
```