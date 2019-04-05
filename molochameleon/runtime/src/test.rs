// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License
#[cfg(test)]

use super::*;
use mock::{Dao, System, Test, ExtBuilder}; // left out ExtBuilder (for now?)
use runtime_io::with_externalities;
use srml_support::{
	assert_noop, assert_ok, assert_err,
	traits::{LockableCurrency, LockIdentifier, WithdrawReason, WithdrawReasons,
	Currency, MakePayment, ReservableCurrency} // don't need all these...
};