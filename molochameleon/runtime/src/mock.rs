// Copyright 2019 Amar Singh
// This file is part of MoloChameleon, licensed with the MIT License
#[cfg(test)]

use runtime_io::with_externalities;
use primitives::{H256, Blake2Hasher};
use support::{impl_outer_origin, assert_ok};
use runtime_primitives::{
    BuildStorage,
    traits::{BlakeTwo256, IdentityLookup},
    testing::{Digest, DigestItem, Header}
};
use crate::{GenesisConfig, Module, Trait}

impl_outer_origin! {
    pub enum Origin for Test {}
}

#[derive(Clone, Eq, PartialEq)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = u64;
    type Lookup = IdentityLookup<u64>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

// may not need, but I am using `balances`
impl balances::Trait for Test {
	type Balance = u64;
	type OnFreeBalanceZero = Staking;
	type OnNewAccount = ();
	type Event = ();
	type TransactionPayment = ();
	type TransferPayment = ();
	type DustRemoval = ();
}

impl Trait for Test {
    type Currency = balances::Module<Test>;
    type BalanceOf = balances::Module<Test>; // different?
    type Event = ();
}

// consider setting up ExtBuilder using https://github.com/paritytech/substrate/blob/master/srml/staking/src/mock.rs
pub struct ExtBuilder {
    voting_period: u32,  // check T::BlockNumber type again
    grace_period: u32,   // ""
    abort_window: u32,   // ""
    proposal_bond: u32,
    dilution_bound: u32,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			voting_period: 7,
			grace_period: 7,
			abort_window: 2,
			proposal_bond: 1,
            dilution_bound: 3,
		}
	}
}
impl ExtBuilder {
    pub fn voting_period(mut self, voting_period: u32) -> Self {
		self.voting_period = voting_period;
		self
	}
    pub fn grace_period(mut self, grace_period: u32) -> Self {
		self.grace_period = grace_period;
		self
	}
    pub fn abort_window(mut self, abort_window: u32) -> Self {
		self.abort_window = abort_window;
		self
	}
    pub fn proposal_bond(mut self, proposal_bond: u32) -> Self {
		self.proposal_bond = proposal_bond;
		self
	}
    pub fn dilution_bound(mut self, dilution_bound: u32) -> Self {
		self.dilution_bound = dilution_bound;
		self
	}
    pub fn build(self) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(GenesisConfig::<Test> {
			voting_period: self.voting_period,
			grace_period: self.grace_period,
			abort_window: self.abort_window,
			proposal_bond: self.proposal_bond,
			dilution_bound: self.dilution_bound,
		}.build_storage().unwrap().0);
		t.into()
	}
}

pub type System = system::Module<Test>;
pub type Dao = Module<Test>;