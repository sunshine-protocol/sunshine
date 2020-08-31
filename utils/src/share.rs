use crate::traits::{
    AccessGenesis,
    AccessProfile,
    VerifyShape,
};
use frame_support::Parameter;
use parity_scale_codec::{
    Decode,
    Encode,
};
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct SharePortion<Shares, FineArithmetic> {
    total: Shares,
    portion: FineArithmetic,
}

impl<Shares: Copy, FineArithmetic: Copy> SharePortion<Shares, FineArithmetic> {
    pub fn total(&self) -> Shares {
        self.total
    }
    pub fn portion(&self) -> FineArithmetic {
        self.portion
    }
}

#[derive(PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub enum ProfileState {
    Locked,
    Unlocked,
}

#[derive(new, PartialEq, Eq, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// share profile reserves the total share amount every time but (might) have a limit on total reservations
pub struct ShareProfile<Id, Shares, State> {
    id: Id,
    /// The total number of shares owned by this participant
    total: Shares,
    /// Tells us if the shares can be used in another vote
    state: State,
}

impl<
        Id: Clone,
        Shares: Copy
            + Default
            + Parameter
            + sp_std::ops::Add<Output = Shares>
            + sp_std::ops::Sub<Output = Shares>
            + Zero
            + From<u32>,
    > ShareProfile<Id, Shares, ProfileState>
{
    pub fn id(&self) -> Id {
        self.id.clone()
    }
    pub fn total(&self) -> Shares {
        self.total
    }

    pub fn is_zero(&self) -> bool {
        self.total == Shares::zero()
    }

    pub fn new_shares(
        id: Id,
        total: Shares,
    ) -> ShareProfile<Id, Shares, ProfileState> {
        ShareProfile {
            id,
            total,
            state: ProfileState::Unlocked,
        }
    }

    pub fn add_shares(
        self,
        amount: Shares,
    ) -> ShareProfile<Id, Shares, ProfileState> {
        let total = self.total + amount;
        ShareProfile { total, ..self }
    }

    pub fn subtract_shares(
        self,
        amount: Shares,
    ) -> ShareProfile<Id, Shares, ProfileState> {
        let total = self.total - amount;
        ShareProfile { total, ..self }
    }

    pub fn lock(self) -> ShareProfile<Id, Shares, ProfileState> {
        ShareProfile {
            state: ProfileState::Locked,
            ..self
        }
    }

    pub fn unlock(self) -> ShareProfile<Id, Shares, ProfileState> {
        ShareProfile {
            state: ProfileState::Unlocked,
            ..self
        }
    }

    pub fn is_unlocked(&self) -> bool {
        matches!(self.state, ProfileState::Unlocked)
    }
}

impl<Id: Clone, Shares: Copy + sp_std::ops::AddAssign + Zero>
    AccessProfile<Shares> for ShareProfile<Id, Shares, ProfileState>
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
        let mut dg = genesis;
        dg.dedup();
        for account_shares in dg.clone() {
            total += account_shares.1;
        }
        WeightedVector { total, vec: dg }
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
