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

/// The AccountId alias in this test module.
pub type AccountIdType = u64;

impl_outer_origin! {
    pub enum Origin for Test {}
}

// Workaround for https://github.com/rust-lang/rust/issues/26925 . Remove when sorted.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Test;

impl system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type Digest = Digest;
    type AccountId = AccountIdType;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type Log = DigestItem;
}

impl balances::Trait for Test {
	type Balance = u64;
	type OnFreeBalanceZero = Dao;
	type OnNewAccount = ();
	type Event = ();
	type TransactionPayment = ();
	type TransferPayment = ();
	type DustRemoval = ();
}

impl Trait for Test {
    type Currency = balances::Module<Self>;
    type BalanceOf = balances::Module<Test>; // different?
    type Event = ();
	// type OnRewardMinted = ();
	// type Slash = ();
	// type Reward = ();
}

// following conventions of https://github.com/paritytech/substrate/blob/master/srml/staking/src/mock.rs
pub struct ExtBuilder {
	existential_deposit: u64,
    voting_period: u32,
    grace_period: u32,
    abort_window: u32,
	proposal_fee: u32,
    proposal_bond: u32,
    dilution_bound: u32,
	member_count: u32,
	pool_address: u32,
}
impl Default for ExtBuilder {
	fn default() -> Self {
		Self {
			existential_deposit: 0,
			voting_period: 7,
			grace_period: 7,
			abort_window: 2,
			proposal_fee: 5,
			proposal_bond: 5,
            dilution_bound: 3,
			member_count: 5,
			total_shares: 20,
			pool_address: 69,
			pool_balance: 48,
		}
	}
}
impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}
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
	pub fn proposal_fee(mut self, proposal_fee: u32) -> Self {
		self.proposal_fee = proposal_fee;
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
	pub fn member_count(mut self, member_count: u32) -> Self {
		self.member_count = member_count;
		self
	}
	pub fn total_shares(mut self, total_shares: u32) -> Self {
		self.total_shares = total_shares;
		self
	}
	pub fn pool_address(mut self, pool_address: )
    pub fn build(self) -> runtime_io::TestExternalities<Blake2Hasher> { // why is the trait bound Blake2Hasher here?
		let (mut t, mut c) = system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let _ = balances::GenesisConfig::<Test>{
			balances: vec![ // not related to share count; used for proposal bonds only!
				(6, 10),		// member
				(7, 20),		// member
				(9, 10),		// member
				(10, 3),		// member
				(12, 7),		// member
				(23, 50),		// proposer
				(24, 100),		// proposer
				(32, 17),		// proposer
				(33, 25),		// proposer
				(34, 44),		// proposer
				(69, 48),		// pool (pool_address, pool_funds)
			],
			transaction_base_fee: 0,
			transaction_byte_fee: 0,
			existential_deposit: self.existential_deposit,
			transfer_fee: 0,
			creation_fee: 0,
			vesting: vec![],
		}.assimilate_storage(&mut t, &mut c);
		let _ = GenesisConfig::<Test>{
			voting_period: self.voting_period;
			grace_period: self.grace_period;
			abort_window: self.abort_window;
			proposal_fee: self.proposal_fee;
			proposal_bond: self.proposal_bond;
			dilution_bound: self.dilution_bound;
			members: vec![
				(6, 4),
				(7, 6),
				(9, 2),
				(10, 2),
				(12, 6),
			],
			applicants: vec![
				(23, 8, 50),
				(24, 15, 100),
				(32, 10, 0),
				(33, 6, 20),
				(34, 8, 0),
			],
			pool: (69, 48),
			member_count: self.member_count,
			pool_address: self.pool_address,
			pool_funds: self.pool_funds,
		}.assimilate_storage(&mut t, &mut c);
		t.into()
	}
}

pub type System = system::Module<Test>;
pub type Balances = balances::Module<Test>;
pub type Dao = Module<Test>;