use crate::traits::{
    AccessGenesis,
    AccessProfile,
    VerifyShape,
};
use codec::{
    Decode,
    Encode,
};
use frame_support::Parameter;
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ProfileState {
    Locked,
    Unlocked,
}

#[derive(new, PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// share profile reserves the total share amount every time but (might) have a limit on total reservations
pub struct ShareProfile<Shares, State> {
    /// The total number of shares owned by this participant
    total: Shares,
    /// Tells us if the shares can be used in another vote
    state: State,
}

impl<
        Shares: Copy
            + Default
            + Parameter
            + sp_std::ops::Add<Output = Shares>
            + sp_std::ops::Sub<Output = Shares>
            + Zero
            + From<u32>,
    > Default for ShareProfile<Shares, ProfileState>
{
    /// The default is 1 shares for convenient usage of the vote module for flat votes
    fn default() -> ShareProfile<Shares, ProfileState> {
        ShareProfile {
            total: Shares::zero() + 1u32.into(),
            state: ProfileState::Unlocked,
        }
    }
}

impl<
        Shares: Copy
            + Default
            + Parameter
            + sp_std::ops::Add<Output = Shares>
            + sp_std::ops::Sub<Output = Shares>
            + Zero
            + From<u32>,
    > ShareProfile<Shares, ProfileState>
{
    pub fn total(&self) -> Shares {
        self.total
    }

    pub fn is_zero(&self) -> bool {
        self.total == Shares::zero()
    }

    pub fn new_shares(total: Shares) -> ShareProfile<Shares, ProfileState> {
        ShareProfile {
            total,
            ..Default::default()
        }
    }

    pub fn add_shares(
        self,
        amount: Shares,
    ) -> ShareProfile<Shares, ProfileState> {
        let total = self.total + amount;
        ShareProfile { total, ..self }
    }

    pub fn subtract_shares(
        self,
        amount: Shares,
    ) -> ShareProfile<Shares, ProfileState> {
        let total = self.total - amount;
        ShareProfile { total, ..self }
    }

    pub fn lock(self) -> ShareProfile<Shares, ProfileState> {
        ShareProfile {
            state: ProfileState::Locked,
            ..self
        }
    }

    pub fn unlock(self) -> ShareProfile<Shares, ProfileState> {
        ShareProfile {
            state: ProfileState::Unlocked,
            ..self
        }
    }

    pub fn is_unlocked(&self) -> bool {
        matches!(self.state, ProfileState::Unlocked)
    }
}

impl<Shares: Copy + sp_std::ops::AddAssign + Zero> AccessProfile<Shares>
    for ShareProfile<Shares, ProfileState>
{
    fn total(&self) -> Shares {
        self.total
    }
}

#[derive(PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
/// The account ownership for the share genesis
pub struct WeightedVector<S, T> {
    total: T,
    vec: Vec<(S, T)>,
}

impl<
        AccountId: Clone,
        Shares: Copy + sp_std::ops::AddAssign + Zero + PartialEq,
    > AccessGenesis<AccountId, Shares> for WeightedVector<AccountId, Shares>
{
    fn total(&self) -> Shares {
        self.total
    }
    fn vec(&self) -> Vec<(AccountId, Shares)> {
        self.vec.clone()
    }
}

impl<
        AccountId: Parameter,
        Shares: Copy + sp_std::ops::AddAssign + Zero + PartialEq,
    > From<Vec<(AccountId, Shares)>> for WeightedVector<AccountId, Shares>
{
    fn from(
        genesis: Vec<(AccountId, Shares)>,
    ) -> WeightedVector<AccountId, Shares> {
        let mut total: Shares = Shares::zero();
        let mut dedup_genesis = genesis;
        dedup_genesis.dedup(); // deduplicated
        for account_shares in dedup_genesis.clone() {
            total += account_shares.1;
        }
        WeightedVector {
            total,
            vec: dedup_genesis,
        }
    }
}

impl<
        AccountId: Parameter,
        Shares: Copy + sp_std::ops::AddAssign + Zero + PartialEq,
    > VerifyShape for WeightedVector<AccountId, Shares>
{
    fn verify_shape(&self) -> bool {
        let mut sum: Shares = Shares::zero();
        for ac in self.vec.iter() {
            sum += ac.1
        }
        sum == self.total
    }
}
