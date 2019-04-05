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

fn new_test_ext() -> runtime_io::TestExternalities<Blake2Hasher> {
    system::GenesisConfig::<Test>::default().build_storage().unwrap().0.into()
}

#[test]
fn it_works_for_default_value() {
    with_externalities(&mut new_test_ext(), || {
        assert_ok!(true, "this will never print");
    });
}

pub type System = system::Module<Test>;
pub type Dao = Module<Test>;