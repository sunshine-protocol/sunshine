use crate::traits::{AccessGenesis, AccessProfile, VerifyShape};
use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::{traits::Zero, RuntimeDebug};
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub enum ShareIdTypes {
    Flat,
    Weighted,
}

impl Default for ShareIdTypes {
    fn default() -> Self {
        ShareIdTypes::Flat
    }
}

#[derive(PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub enum ShareID<Id: Codec + PartialEq + Zero + From<u32> + Copy> {
    Flat(Id),
    Weighted(Id),
}

impl<Id: Codec + PartialEq + Zero + From<u32> + Copy> Default for ShareID<Id> {
    fn default() -> Self {
        ShareID::Flat(Id::zero())
    }
}

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// share profile reserves the total share amount every time but (might) have a limit on total reservations
pub struct ShareProfile<Shares> {
    /// The total number of shares owned by this participant
    total: Shares,
    /// The reference count for the number of votes that this is used, initialized at 0
    times_reserved: u32,
    /// Tells us if the shares can be used in another vote
    locked: bool,
}

impl<
        Shares: Copy
            + Default
            + Parameter
            + sp_std::ops::Add<Output = Shares>
            + sp_std::ops::Sub<Output = Shares>
            + Zero,
    > ShareProfile<Shares>
{
    pub fn total(&self) -> Shares {
        self.total
    }

    pub fn times_reserved(&self) -> u32 {
        self.times_reserved
    }

    pub fn is_zero(&self) -> bool {
        self.total == Shares::zero()
    }

    pub fn new_shares(total: Shares) -> ShareProfile<Shares> {
        ShareProfile {
            total,
            ..Default::default()
        }
    }

    pub fn add_shares(self, amount: Shares) -> ShareProfile<Shares> {
        let total = self.total + amount;
        ShareProfile { total, ..self }
    }

    pub fn subtract_shares(self, amount: Shares) -> ShareProfile<Shares> {
        let total = self.total - amount;
        ShareProfile { total, ..self }
    }

    pub fn increment_times_reserved(self) -> ShareProfile<Shares> {
        let times_reserved = self.times_reserved + 1u32;
        ShareProfile {
            times_reserved,
            ..self
        }
    }

    pub fn decrement_times_reserved(self) -> ShareProfile<Shares> {
        let times_reserved = self.times_reserved - 1u32;
        ShareProfile {
            times_reserved,
            ..self
        }
    }

    pub fn lock(self) -> ShareProfile<Shares> {
        ShareProfile {
            locked: true,
            ..self
        }
    }

    pub fn unlock(self) -> ShareProfile<Shares> {
        ShareProfile {
            locked: false,
            ..self
        }
    }

    pub fn is_unlocked(&self) -> bool {
        !self.locked
    }
}

impl<Shares: Copy + Parameter> AccessProfile<Shares> for ShareProfile<Shares> {
    fn total(&self) -> Shares {
        self.total
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The account ownership for the share genesis
pub struct SimpleShareGenesis<AccountId, Shares> {
    total: Shares,
    account_ownership: Vec<(AccountId, Shares)>,
}

impl<AccountId: Clone, Shares: Parameter + From<u32>> AccessGenesis<AccountId, Shares>
    for SimpleShareGenesis<AccountId, Shares>
{
    fn total(&self) -> Shares {
        self.total.clone()
    }
    fn account_ownership(&self) -> Vec<(AccountId, Shares)> {
        self.account_ownership.clone()
    }
}

impl<AccountId: Parameter, Shares: Parameter + From<u32> + sp_std::ops::AddAssign>
    From<Vec<(AccountId, Shares)>> for SimpleShareGenesis<AccountId, Shares>
{
    fn from(genesis: Vec<(AccountId, Shares)>) -> SimpleShareGenesis<AccountId, Shares> {
        let mut total: Shares = 0u32.into();
        let mut dedup_genesis = genesis;
        dedup_genesis.dedup(); // deduplicated
        for account_shares in dedup_genesis.clone() {
            total += account_shares.1;
        }
        SimpleShareGenesis {
            total,
            account_ownership: dedup_genesis,
        }
    }
}

impl<
        AccountId: Parameter,
        Shares: Copy + Parameter + From<u32> + sp_std::ops::Add<Output = Shares>,
    > VerifyShape for SimpleShareGenesis<AccountId, Shares>
{
    fn verify_shape(&self) -> bool {
        // TODO: clean up and optimize
        let mut sum: Shares = 0u32.into();
        for ac in self.account_ownership.iter() {
            sum = sum + ac.1
        }
        sum == self.total
    }
}
