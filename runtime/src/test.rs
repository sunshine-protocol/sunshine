// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License
#[cfg(test)]

use super::*;
use runtime_io::with_externalities;
use srml_support::{
	assert_noop, assert_ok, assert_err, assert_eq_uvec,
	traits::{Currency, LockableCurrency, ReservableCurrency} // remove unused imports...
};
use mock::{Dao, System, Test, ExtBuilder};

// genesis config
#[test]
fn genesis_config_works() {
	// verify initial conditions of mock.rs
	with_externalities(&mut ExtBuilder::default()
		.build(), || {
		//

	}
	// instantiate three members at the DAO's initialization

	// check the correct initialization of required maps
}

// TODO
// model-based testing (proptest and quickcheck)

// NEED
// test successful execution or each function emits the correct event
//
// test that an applicant cannot add an application if they have a pending application
//
// test that abort works within the window -- same for all windows (vote -> Voting; rageQuit -> Grace)
// test that abort doesn't work outside the window -- same for all windows (vote -> Voting; rageQuit -> Grace
//
// CHECK that all Pool fields are updated appropriately (haven't done this yet)
// (1) proposal is processed => balance is increased by tokenTribute; shares increase by shares
// (2) member ragequits => balance is decreased by set amount; shares decrease by number of member shares
//
/// CONVERSION
// test conversion between `BalanceOf<T>` and `Balance`
//
// CHECK rageQuit -> Grace doesn't work if there is a pending yesVote
//
// CHECK that dependent maps are updated at the correct state transitions
//
// ADD CODE && TDD
// test that the processer is not the proposer
// test that reward parameterizations are not an attack vector
//
// EXISTING BUGS
// -- use of `BalanceOf` (use the staking module); the encoding within `decl_storage` is particularly annoying
// -- `<Proposals<T>>` is not updated correctly
// -- economic security (collusion risk is not covered)
//
// WANT
// fuzzing
// tool for checking that no panics can occur after changes to storage (like concolic execution)