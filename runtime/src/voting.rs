// Copyright 2019 Amar Singh
// This file is part of Sunshine, licensed with the MIT License

// Voting Algorithms
// --> provide default implementations of each
//     but making them into traits allows them to be overwritten
trait VoteParadime {
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

    type item Voters: Vec<_>;

    // use all the other function definitions to 
    fn build () {
        unimplemented!();
        // have this take the form of required by the Pool struct..,
    }

}