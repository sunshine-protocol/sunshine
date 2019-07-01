// Copyright 2019 Amar Singh
// This file is part of Sunshine, licensed with the MIT License

// TODO
// define Application
// identify different function signatures...
trait moloch<AccountId, BalanceOf> {
	type Error: some_err_enum;

	pub fn propose(origin, Option<Box<Application>>) -> Result<Application, Error>;

	pub fn abort(origin, Option<Box<Application>>) -> Result<Application, Error>;

	pub fn vote(origin, Option<Box<Application>>) -> Result<Application, Error>;

	pub fn process(origin, Option<Box<Application>>) -> Result<Application, Error>;

	pub fn exit(origin, Option<Box<Application>>) -> Result<Application, Error>;
}

/// ----------------------referendum logic ----------------------
//! Brainstorming (pools ~= threadpools in this *context*)
//! What if every proposal was an object that was voted on in a pool 
//! Once it passed, it would be sent to a different pool where it *might* get processed
//! (1) The first pool is basically tasked with adding up votes (according to algorithm chosen), 
//! passing successful proposals to the second pool, and discarding stale, unpassed proposals.
//! (2) The second pool is tasked with checks on dilution/safety, 
//! discarding stale proposals (*requires thought*), and executing successful proposals
//! 
//! For both pools (1) and (2) also require significant reads to check pending proposals supported 
//! ============================================================> to verify ragequit authorization

// spawn a future to follow a specific proposal referendum?
// is there one proposal per referendum? Yes.

pub trait Setting {
    // algorithm
    // sybil
    // threshold
}

/// Referendum
/// a lightweight handle like `Arc`
pub trait election: Clone {

    // the error type for interacting with the Referendum
    type Error: std::fmt::Debug + Send;

    // A stream that yields the new votes for the given referendum
    type VoteUpdates: Stream<Item=VoteUpdate,Error=Self::Error> + Send;

    // A referendum's configuration
    type Settings: Setting + Send;
    
    // get a stream of vote updates
    fn vote_updates(&self, vote_id: u64) -> Self::VoteUpdates;

    // configure voting algorithm
    fn config<A, S, T>(&self, algorithm: A, sybil: S, threshold: T) -> Self::VoteUpdate;
}

// OBJECTIVE: abstract all voting functionality
//
// Algos:
// --> Adaptive Quorum Biasing
// --> Quadratic Voting                             // create a RadLib branch and implement Liberal Radicalism using this...
// --> Holographic Consensus (prediction markets)?
// --> Conviction Voting?
//
// Sybil Mechanism:
// --> stake-weighted voting
// --> 1P1V
// --> proof of membership of a *club*              // this could allow us to verify off-chain stuff and create verifiable groups)
//
// Thresholds:
// --> dynamic (AQP)
// --> set_threshold()                              // means different things depending on which algo is invoked
// --> majority 
// --> supermajority
