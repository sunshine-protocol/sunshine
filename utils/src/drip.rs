use codec::{
    Decode,
    Encode,
};
use sp_runtime::{
    traits::Zero,
    RuntimeDebug,
};
use sp_std::prelude::*;

#[derive(
    new, PartialEq, Eq, Default, Clone, Copy, Encode, Decode, RuntimeDebug,
)]
pub struct DripRate<BlockNumber, Currency> {
    amount: Currency,
    period_length: BlockNumber,
}

impl<
        BlockNumber: Copy + PartialEq + Zero,
        Currency: Copy + PartialEq + Zero,
    > DripRate<BlockNumber, Currency>
{
    pub fn amount(&self) -> Currency {
        self.amount
    }
    pub fn period_length(&self) -> BlockNumber {
        self.period_length
    }
}

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
pub struct Drip<AccountId, Rate> {
    source: AccountId,
    destination: AccountId,
    rate: Rate,
}

impl<AccountId: Clone, Rate: Copy> Drip<AccountId, Rate> {
    pub fn source(&self) -> AccountId {
        self.source.clone()
    }
    pub fn destination(&self) -> AccountId {
        self.destination.clone()
    }
    pub fn rate(&self) -> Rate {
        self.rate
    }
}
