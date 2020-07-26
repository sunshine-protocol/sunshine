#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use frame_support::{
    decl_event,
    decl_module,
    decl_storage,
    traits::{
        Currency,
        Get,
    },
};
use frame_system::{self as system,};
use sp_runtime::{
    traits::{
        AccountIdConversion,
        Zero,
    },
    ModuleId,
};

type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as system::Trait>::AccountId,
>>::Balance;

pub trait Trait: system::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    /// The currency type
    type Currency: Currency<Self::AccountId>;
    /// Where the conditional taxes go
    type TreasuryAddress: Get<ModuleId>;
}

decl_event!(
    pub enum Event<T> where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::BlockNumber,
        Balance = BalanceOf<T>,
    {
        // TODO: I would like for this to emit the rate of minting as well?
        TreasuryMinting(Balance, BlockNumber, AccountId),
    }
);

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event() = default;

        /// Mint funds for the treasury
        fn on_finalize(_n: T::BlockNumber) {
            if <system::Module<T>>::block_number() % Self::minting_interval() == Zero::zero() {
                let mint_amt = Self::mint_amount();
                let treasury_id = Self::account_id();
                T::Currency::deposit_creating(&treasury_id, mint_amt);
                Self::deposit_event(RawEvent::TreasuryMinting(
                    T::Currency::free_balance(&treasury_id),
                    <system::Module<T>>::block_number(),
                    treasury_id)
                );
            }
        }
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Treasury {
        /// Interval in number of blocks to mint to the treasury
        pub MintingInterval get(fn minting_interval) config(): T::BlockNumber;
        /// Minting amount
        pub MintAmount get(fn mint_amount) config(): BalanceOf<T>;
    }
}

impl<T: Trait> Module<T> {
    /// The account ID of this module's treasury
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::TreasuryAddress::get().into_account()
    }
}
