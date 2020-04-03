use crate::traits::VerifyShape;
use codec::{Decode, Encode, FullCodec};
use frame_support::Parameter;
use sp_runtime::{
    traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero},
    RuntimeDebug,
};
use sp_std::{fmt::Debug, prelude::*};

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Atomic share profile reserves the total share amount every time but (might) have a limit on total reservations
pub struct AtomicShareProfile<Shares> {
    /// The total number of shares owned by this participant
    total: Shares,
    /// The reference count for the number of votes that this is used, initialized at 0
    times_reserved: u32,
    /// Tells us if the shares can be used in another vote
    locked: bool,
}

impl<
        Shares: Parameter
            + Member
            + AtLeast32Bit
            + FullCodec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + Zero,
    > Default for AtomicShareProfile<Shares>
{
    fn default() -> Self {
        AtomicShareProfile {
            total: 0u32.into(),
            times_reserved: 0u32,
            locked: false,
        }
    }
}

impl<
        Shares: Parameter
            + Member
            + AtLeast32Bit
            + FullCodec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + Zero,
    > AtomicShareProfile<Shares>
{
    pub fn get_shares(&self) -> Shares {
        self.total
    }

    pub fn get_times_reserved(&self) -> u32 {
        self.times_reserved
    }

    pub fn new_shares(total: Shares) -> AtomicShareProfile<Shares> {
        AtomicShareProfile {
            total,
            ..Default::default()
        }
    }

    pub fn add_shares(self, amount: Shares) -> AtomicShareProfile<Shares> {
        let total = self.total + amount;
        AtomicShareProfile { total, ..self }
    }

    pub fn subtract_shares(self, amount: Shares) -> AtomicShareProfile<Shares> {
        let total = self.total - amount;
        AtomicShareProfile { total, ..self }
    }

    pub fn iterate_times_reserved(self, amount: u32) -> AtomicShareProfile<Shares> {
        let times_reserved = self.times_reserved + amount;
        AtomicShareProfile {
            times_reserved,
            ..self
        }
    }

    pub fn decrement_times_reserved(self, amount: u32) -> AtomicShareProfile<Shares> {
        let times_reserved = self.times_reserved - amount;
        AtomicShareProfile {
            times_reserved,
            ..self
        }
    }

    pub fn lock(self) -> AtomicShareProfile<Shares> {
        AtomicShareProfile {
            locked: true,
            ..self
        }
    }

    pub fn unlock(self) -> AtomicShareProfile<Shares> {
        AtomicShareProfile {
            locked: false,
            ..self
        }
    }

    pub fn is_unlocked(&self) -> bool {
        !self.locked
    }
}

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
