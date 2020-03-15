use crate::traits::VerifyShape;
use codec::{Decode, Encode};
use frame_support::Parameter;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Copy, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The share profile stores information regarding share reservation in the context of
/// collateralized actions
pub struct ShareProfile<Shares> {
    pub free: Shares,
    pub reserved: Shares,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The account ownership for the share genesis
pub struct SimpleShareGenesis<AccountId, Shares> {
    pub total: Shares,
    pub account_ownership: Vec<(AccountId, Shares)>,
}

impl<AccountId: Parameter, Shares: Parameter + Zero> From<Vec<(AccountId, Shares)>>
    for SimpleShareGenesis<AccountId, Shares>
{
    fn from(genesis: Vec<(AccountId, Shares)>) -> SimpleShareGenesis<AccountId, Shares> {
        let mut total = Shares::zero();
        for account_shares in genesis.clone() {
            total = total + account_shares.1.clone();
        }
        SimpleShareGenesis {
            total,
            account_ownership: genesis,
        }
    }
}

impl<AccountId: Parameter, Shares: Parameter + Zero> VerifyShape
    for SimpleShareGenesis<AccountId, Shares>
{
    fn verify_shape(&self) -> bool {
        // TODO: clean up and optimize
        let mut sum = Shares::zero();
        for ac in self.account_ownership.iter() {
            sum = sum + ac.1.clone()
        }
        sum == self.total
    }
}
