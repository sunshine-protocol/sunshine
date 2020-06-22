#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This bank module is for gradually streaming capital from sender { AccountId, Org } to recipient { Org } so that withdrawal rules respect/enforce the ownership structure of the Org

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    traits::ExistenceRequirement,
    traits::{Currency, Get},
    Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero}, // CheckedAdd, CheckedSub
    DispatchResult,
    Permill,
};
use sp_std::{fmt::Debug, prelude::*};
use util::{
    bank::{BankState, OnChainTreasuryID, Sender, TransferInformation},
    traits::{GenerateUniqueID, GroupMembership, IDIsAvailable, Increment, SeededGenerateUniqueID},
};

/// The balances type for this module
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: frame_system::Trait + org::Trait {
    /// The overarching event types
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The identifier for transfers, used to limit withdrawals by individual AccountIds from OrgIds by ownership
    type TransferId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    /// The minimum amount necessary to use this module for this transfer
    type MinimumTransfer: Get<BalanceOf<Self>>;

    /// The minimum amount necessary to open an organizational bank account
    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as Trait>::TransferId,
        Balance = BalanceOf<T>,
    {
        AccountOpensOrgBankAccount(AccountId, OnChainTreasuryID, Balance, OrgId, Option<AccountId>),
        AccountToOrgTransfer(TransferId, AccountId, OrgId),
        OrgToOrgTransfer(TransferId, OrgId, OrgId),
    } // to add gradually, reservations, useful for collateralizing loans
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotOpenBankAccountForOrgIfNotOrgMember,
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Court {
        /// Counter for generating unique bank accounts
        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        /// Counter for transfers to the OnChainTreasuryID
        TransferNonceMap get(fn transfer_nonce_map): map
            hasher(blake2_128_concat) OnChainTreasuryID => T::TransferId;

        /// Transfer info, must be referenced for every withdrawal from Org for AccountId
        pub TransferInfo get(fn transfer_info): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::TransferId =>
            Option<TransferInformation<T::AccountId, T::OrgId, BalanceOf<T>>>; // should have the amount

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique `OnChainTreasuryID`s
        pub BankStores get(fn bank_stores): map
            hasher(blake2_128_concat) OnChainTreasuryID =>
            Option<BankState<T::AccountId, T::OrgId, BalanceOf<T>>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn account_opens_account_for_org_with_deposit(origin, org: T::OrgId, deposit_amount: BalanceOf<T>, controller: Option<T::AccountId>) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            // auth? at least ensure that they are a member of the Org
            let authentication = <org::Module<T>>::is_member_of_group(org, &opener);
            ensure!(authentication, Error::<T>::CannotOpenBankAccountForOrgIfNotOrgMember);
            ensure!(deposit_amount >= T::MinimumInitialDeposit::get(), Error::<T>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum);

            // generate the unique identifier
            let new_treasury_id = Self::generate_unique_id();
            // make the transfer
            T::Currency::transfer(&opener, &Self::account_id(new_treasury_id), deposit_amount, ExistenceRequirement::KeepAlive)?;
            // create new bank object
            let new_bank = BankState::new_from_deposit(org, deposit_amount, controller.clone());
            // insert new bank object
            <BankStores<T>>::insert(new_treasury_id, new_bank);
            // create new transfer object
            let new_transfer = TransferInformation::new(Sender::Account(opener.clone()), org, deposit_amount, BalanceOf::<T>::zero());
            // generate unique transfer id
            let new_transfer_id = Self::seeded_generate_unique_id(new_treasury_id);
            // insert new transfer
            <TransferInfo<T>>::insert(new_treasury_id, new_transfer_id, new_transfer);
            Self::deposit_event(RawEvent::AccountOpensOrgBankAccount(opener, new_treasury_id, deposit_amount, org, controller));
            Ok(())
        }
        // account to existing org transfer
        #[weight = 0]
        fn account_to_org_transfer(origin, bank_id: OnChainTreasuryID, transfer_amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Ok(())
        }
        // org to org transfer
        // withdrawal by account id from org
    }
}

impl<T: Trait> Module<T> {
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
}

impl<T: Trait> IDIsAvailable<OnChainTreasuryID> for Module<T> {
    fn id_is_available(id: OnChainTreasuryID) -> bool {
        <BankStores<T>>::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, T::TransferId)> for Module<T> {
    fn id_is_available(id: (OnChainTreasuryID, T::TransferId)) -> bool {
        <TransferInfo<T>>::get(id.0, id.1).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<OnChainTreasuryID> for Module<T> {
    fn generate_unique_id() -> OnChainTreasuryID {
        let mut treasury_nonce_id = BankIDNonce::get().increment();
        while !Self::id_is_available(treasury_nonce_id) {
            treasury_nonce_id = treasury_nonce_id.increment();
        }
        BankIDNonce::put(treasury_nonce_id);
        treasury_nonce_id
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::TransferId, OnChainTreasuryID> for Module<T> {
    fn seeded_generate_unique_id(seed: OnChainTreasuryID) -> T::TransferId {
        let mut transfer_nonce = <TransferNonceMap<T>>::get(seed) + 1u32.into();
        while !Self::id_is_available((seed, transfer_nonce)) {
            transfer_nonce = transfer_nonce + 1u32.into();
        }
        <TransferNonceMap<T>>::insert(seed, transfer_nonce);
        transfer_nonce
    }
}
