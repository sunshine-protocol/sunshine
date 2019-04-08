// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License
#[cfg(test)]

use super::*;
use runtime_io::with_externalities;
use srml_support::{
	assert_noop, assert_ok, assert_err, assert_eq_uvec,
	traits::{LockableCurrency, LockIdentifier, WithdrawReason, WithdrawReasons,
	Currency, MakePayment, ReservableCurrency} // don't need all these...
};
use mock::{Dao, System, Test, ExtBuilder};

// NEED
// test successful execution or each function emits the correct event
//
// test that an applicant cannot add an application if they have a pending application
//
// test that abort works within the window -- same for all windows (vote -> Voting; rageQuit -> Grace)
// test that abort doesn't work outside the window -- same for all windows (vote -> Voting; rageQuit -> Grace
//
// also check that rageQuit -> Grace doesn't work if there is a pending yesVote
#[test]




//
// ADD CODE && TDD
// test that the processer is not the proposer
// test that reward parameterizations don't foster an attack vector
// test nomination (allow for delegation-based voting similarly to scale representational participation)
//
//
// EXISTING BUGS
// -- iterator adaptor `all` invocation
// -- use of BalanceOf (use the staking module)
//
// WANT
// fuzzing