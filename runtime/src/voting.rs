// Copyright 2019 Amar Singh
// This file is part of Sunshine, licensed with the MIT License
  
//! Brainstorming (pools ~= threadpools in this *context*)
//! What if every proposal was an object that was voted on in a pool 
//! Once it passed, it would be sent to a different pool where it *might* get processed
//! (1) The first pool is basically tasked with adding up votes (according to algorithm chosen), 
//! passing successful proposals to the second pool, and discarding stale, unpassed proposals.
//! (2) The second pool is tasked with checks on dilution/safety, 
//! discarding stale proposals (*requires thought*), and executing successful proposals
//! 
//! For both pools (1) and (2) also require significant reads to check pending proposal support 
//! ====> ragequit authorization

/// Referendum
/// a lightweight handle like an `Arc`
pub trait Referendum: Clone {

    // the error type for interacting with the Referendum
    type Error: std::fmt::Debug + Send;

    // A stream that yields the new votes for the given referendum
    type VoteUpdates: Stream<Item=VoteUpdate,Error=Self::Error> + Send;
    
    // get a stream of vote updates
    fn vote_updates(&self, vote_id: u64) -> Self::VoteUpdates;

}

// spawn a future to follow a specific proposal referendum
// is there one proposal per referendum

// OBJECTIVE: abstract all voting functionality
    // provide generic implementations for the following:
    // algos:
    // --> Adaptive Quorum Biasing
    // --> Quadratic Voting
    // --> Continuous Signalling Mechanisms
    // ----> Conviction Voting 
    // sybil mechanism:
    // --> stake-weighted voting
    // --> 1P1V
    // thresholds:
    // --> dynamic (AQP)
    // --> set()
    // --> majority 
    // --> supermajority