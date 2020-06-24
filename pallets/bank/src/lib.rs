#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This bank module is for gradually streaming capital from sender { AccountId, Org }
//! to recipient { Org } so that withdrawal rules respect/enforce the
//! ownership structure of the Org

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
    },
    Parameter,
};
use frame_system::{
    self as system,
    ensure_signed,
};
use sp_runtime::{
    traits::{
        AccountIdConversion,
        AtLeast32Bit,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchError,
    DispatchResult,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    bank::{
        BankState,
        OnChainTreasuryID,
        Sender,
        TransferInformation,
    },
    traits::{
        CalculateOwnership,
        DepositSpendOps,
        GenerateUniqueID,
        GroupMembership,
        IDIsAvailable,
        Increment,
        PostTransfer,
        RegisterOrgAccount,
        SeededGenerateUniqueID,
    },
};

/// The balances type for this module
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

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
        AccountOpensOrgBankAccount(AccountId, OnChainTreasuryID, TransferId, Balance, OrgId, Option<AccountId>),
        AccountToOrgTransfer(TransferId, AccountId, OnChainTreasuryID, Balance),
        OrgToOrgTransfer(TransferId, AccountId, OnChainTreasuryID, OnChainTreasuryID, Balance),
        // balance is amount withdrawn
        WithdrawalFromOrgToAccount(AccountId, OnChainTreasuryID, TransferId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        AccountHasNoOwnershipInOrg,
        TransferMustExceedModuleMinimum,
        CannotOpenBankAccountForOrgIfNotOrgMember,
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        BankAccountMustExistToPostTransfer,
        SignerNotAuthorizedToTransferOnBehalfOfOrg,
        WithdrawalFromOrgToAccountRequiresExistingOrgBankAccount,
        WithdrawalFromOrgToAccountRequiresValidTransferReference,
        RequestExceedsAmountThatCanBeWithdrawnForAccountFromOrg,
        WithdrawalRequestExceedsUnClaimedTransferAmount,
        WithdrawalRequestExceedsFreeOrgBankFunds,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Counter for generating unique bank accounts
        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        /// Counter for transfers to the OnChainTreasuryID
        TransferNonceMap get(fn transfer_nonce_map): map
            hasher(blake2_128_concat) OnChainTreasuryID => T::TransferId;

        /// Transfer info, must be referenced for every withdrawal from Org for AccountId
        pub TransferInfo get(fn transfer_info): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::TransferId =>
            Option<TransferInformation<T::AccountId, T::OrgId, BalanceOf<T>>>;

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique `OnChainTreasuryID`s
        pub BankStores get(fn bank_stores): map
            hasher(blake2_128_concat) OnChainTreasuryID =>
            Option<BankState<T::AccountId, T::OrgId, BalanceOf<T>>>;

        /// Initialized upon first withdrawal request s.t. value contains
        /// amount remaining, if any
        pub Withdrawals get(fn withdrawals): double_map
            hasher(blake2_128_concat) (OnChainTreasuryID, T::TransferId),
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn account_opens_account_for_org_with_deposit(origin, org: T::OrgId, deposit_amount: BalanceOf<T>, controller: Option<T::AccountId>) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            // auth checks membership in the Org
            let authentication = <org::Module<T>>::is_member_of_group(org, &opener);
            ensure!(authentication, Error::<T>::CannotOpenBankAccountForOrgIfNotOrgMember);
            ensure!(deposit_amount >= T::MinimumInitialDeposit::get(), Error::<T>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum);
            // register new bank account for org
            let new_treasury_id = Self::generate_unique_id();
            // make transfer from seeder to the new bank account
            let new_transfer_id = Self::post_transfer(opener.clone(), None, new_treasury_id, deposit_amount)?;
            // infallible registration because treasury id was just generated and is unique, qed
            Self::register_org_account(new_treasury_id, org, deposit_amount, controller.clone());
            // event emission
            Self::deposit_event(RawEvent::AccountOpensOrgBankAccount(opener, new_treasury_id, new_transfer_id, deposit_amount, org, controller));
            Ok(())
        }
        // account to existing org transfer
        #[weight = 0]
        fn account_to_org_transfer(origin, bank_id: OnChainTreasuryID, transfer_amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // execute transfer and return new transfer identifier
            let new_transfer_id = Self::post_transfer(sender.clone(), None, bank_id, transfer_amount)?;
            // event emission
            Self::deposit_event(RawEvent::AccountToOrgTransfer(new_transfer_id, sender, bank_id, transfer_amount));
            Ok(())
        }
        // org to org transfer
        #[weight = 0]
        fn org_to_org_transfer(origin, sender_bank_id: OnChainTreasuryID, recipient_bank_id: OnChainTreasuryID, transfer_amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // execute transfer and return new transfer identifier
            let new_transfer_id = Self::post_transfer(sender.clone(), Some(sender_bank_id), recipient_bank_id, transfer_amount)?;
            // event emission
            Self::deposit_event(RawEvent::OrgToOrgTransfer(new_transfer_id, sender, sender_bank_id, recipient_bank_id, transfer_amount));
            Ok(())
        }
        // withdrawal by account id from org
        #[weight = 0]
        fn withdraw_from_org_to_account(origin, bank_id: OnChainTreasuryID, transfer_id: T::TransferId, amount_requested: Option<BalanceOf<T>>) -> DispatchResult {
            let withdrawer = ensure_signed(origin)?;
            let bank_account = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::WithdrawalFromOrgToAccountRequiresExistingOrgBankAccount)?;
            let transfer_info = <TransferInfo<T>>::get(bank_id, transfer_id).ok_or(Error::<T>::WithdrawalFromOrgToAccountRequiresValidTransferReference)?;
            // check if the account has withdrawn from this path before and, if so, how much is left
            let amount_left_for_account_to_withdraw = if let Some(left_amt) = <Withdrawals<T>>::get((bank_id, transfer_id), &withdrawer) {
                // => this call has been made before, here is the amount unclaimed but due
                left_amt
            } else {
                Self::calculate_proportional_amount_for_account(transfer_info.amount_transferred(), withdrawer.clone(), bank_account.org())?
            };
            // amount to claim
            let (amount_to_claim, amount_left_after_withdrawal)  = if let Some(requested_amt) = amount_requested {
                ensure!(requested_amt >= amount_left_for_account_to_withdraw, Error::<T>::RequestExceedsAmountThatCanBeWithdrawnForAccountFromOrg);
                let amount_left_after_withdrawal = requested_amt - amount_left_for_account_to_withdraw;
                (requested_amt, amount_left_after_withdrawal)
            } else {
                (amount_left_for_account_to_withdraw, BalanceOf::<T>::zero())
            };
            // change transfer_info to claim
            let new_transfer_info = transfer_info.claim_amount(amount_to_claim).ok_or(Error::<T>::WithdrawalRequestExceedsUnClaimedTransferAmount)?;
            // change bank to spend from free
            let new_bank = bank_account.spend_from_free(amount_to_claim).ok_or(Error::<T>::WithdrawalRequestExceedsFreeOrgBankFunds)?;
            // make the transfer
            T::Currency::transfer(&withdrawer, &Self::account_id(bank_id), amount_to_claim, ExistenceRequirement::KeepAlive)?;
            // insert updated state
            <BankStores<T>>::insert(bank_id, new_bank);
            <TransferInfo<T>>::insert(bank_id, transfer_id, new_transfer_info);
            <Withdrawals<T>>::insert((bank_id, transfer_id), withdrawer.clone(), amount_left_after_withdrawal);
            Self::deposit_event(RawEvent::WithdrawalFromOrgToAccount(withdrawer, bank_id, transfer_id, amount_to_claim));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: T::OrgId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let proportion_due =
            <org::Module<T>>::calculate_proportion_ownership_for_account(
                account, group,
            )?;
        Ok(proportion_due * amount)
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

impl<T: Trait> SeededGenerateUniqueID<T::TransferId, OnChainTreasuryID>
    for Module<T>
{
    fn seeded_generate_unique_id(seed: OnChainTreasuryID) -> T::TransferId {
        let mut transfer_nonce = <TransferNonceMap<T>>::get(seed) + 1u32.into();
        while !Self::id_is_available((seed, transfer_nonce)) {
            transfer_nonce = transfer_nonce + 1u32.into();
        }
        <TransferNonceMap<T>>::insert(seed, transfer_nonce);
        transfer_nonce
    }
}

impl<T: Trait> RegisterOrgAccount<T::OrgId, T::AccountId, BalanceOf<T>>
    for Module<T>
{
    type TreasuryId = OnChainTreasuryID;
    // passed in bank_id must not already be claimed
    fn register_org_account(
        bank_id: OnChainTreasuryID,
        org: T::OrgId,
        deposit_amount: BalanceOf<T>,
        controller: Option<T::AccountId>,
    ) {
        // create new bank object
        let new_bank =
            BankState::new_from_deposit(org, deposit_amount, controller);
        // insert new bank object
        <BankStores<T>>::insert(bank_id, new_bank);
    }
}

impl<T: Trait> PostTransfer<OnChainTreasuryID, T::AccountId, BalanceOf<T>>
    for Module<T>
{
    type TransferId = T::TransferId;
    // recipient bank_id MUST be valid or funds will be lost
    fn post_transfer(
        sender: T::AccountId,
        on_behalf_of: Option<OnChainTreasuryID>,
        bank_id: OnChainTreasuryID,
        amt: BalanceOf<T>,
    ) -> Result<Self::TransferId, DispatchError> {
        ensure!(
            amt >= T::MinimumTransfer::get(),
            Error::<T>::TransferMustExceedModuleMinimum
        );
        let new_transfer = if let Some(sender_bank_id) = on_behalf_of {
            let bank = <BankStores<T>>::get(sender_bank_id)
                .ok_or(Error::<T>::BankAccountMustExistToPostTransfer)?;
            let authentication =
                <org::Module<T>>::is_member_of_group(bank.org(), &sender)
                    || bank.is_controller(&sender);
            ensure!(
                authentication,
                Error::<T>::SignerNotAuthorizedToTransferOnBehalfOfOrg
            );
            T::Currency::transfer(
                &Self::account_id(sender_bank_id),
                &Self::account_id(bank_id),
                amt,
                ExistenceRequirement::KeepAlive,
            )?;
            TransferInformation::new(
                Sender::Org(bank.org()),
                amt,
                BalanceOf::<T>::zero(),
            )
        } else {
            T::Currency::transfer(
                &sender,
                &Self::account_id(bank_id),
                amt,
                ExistenceRequirement::KeepAlive,
            )?;
            TransferInformation::new(
                Sender::Account(sender),
                amt,
                BalanceOf::<T>::zero(),
            )
        };
        // generate unique transfer id
        let new_transfer_id = Self::seeded_generate_unique_id(bank_id);
        // insert new transfer
        <TransferInfo<T>>::insert(bank_id, new_transfer_id, new_transfer);
        Ok(new_transfer_id)
    }
}
