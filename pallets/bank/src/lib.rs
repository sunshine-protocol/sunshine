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
        BankMapId,
        BankState,
        FullBankId,
        OnChainTreasuryID,
        Sender,
        SpendReservation,
        TransferInformation,
        TransferState,
    },
    traits::{
        CalculateOwnership,
        GenerateUniqueID,
        GroupMembership,
        IDIsAvailable,
        Increment,
        PostOrgTransfer,
        PostUserTransfer,
        RegisterOrgAccount,
        ReserveOrgSpend,
        SeededGenerateUniqueID,
        SpendWithdrawOps,
        StopSpendsStartWithdrawals,
        WithdrawFromOrg,
    },
};

/// The balances type for this module
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait: frame_system::Trait + org::Trait {
    /// The overarching event types
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The identifier for transfers and reserves associated with a bank account
    type AssociatedId: Parameter
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
        <T as Trait>::AssociatedId,
        Balance = BalanceOf<T>,
    {
        AccountOpensOrgBankAccount(AccountId, OnChainTreasuryID, AssociatedId, Balance, OrgId, Option<AccountId>),
        AccountToOrgTransfer(AssociatedId, AccountId, OnChainTreasuryID, Balance),
        OrgToOrgTransfer(AssociatedId, AccountId, OnChainTreasuryID, OnChainTreasuryID, Balance),
        // balance is amount withdrawn
        WithdrawalFromOrgToAccount(AccountId, OnChainTreasuryID, AssociatedId, Balance),
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
        CannotReserveOrgSpendIfBankStoreDNE,
        CannotReserveOrgSpendIfTransferDNE,
        ReserveOrgSpendExceedsFreeTransferCapital,
        CannotUnreserveSpendReservationThatDNE,
        CannotUnreserveSpendReservationIfBankStoreDNE,
        CannotUnreserveSpendReservationIfBankReservedIsLessThanAmtToUnreserve,
        CannotTransferFromOrgToOrgIfBankForTransferReferenceDNE,
        CannotTransferFromOrgToOrgIfReferenceTransferDNE,
        CannotTransferFromOrgToOrgIfInWrongStateOrNotEnoughFunds,
        CannotTransferFromOrgToAccountIfBankForTransferReferenceDNE,
        CannotTransferFromOrgToAccountIfReferenceTransferDNE,
        CannotTransferFromOrgToAccountIfInWrongStateOrNotEnoughFunds,
        TransferFailsIfDestBankDNE,
        TransferReservedFailsIfSrcBankDNE,
        TransferReservedFailsIfSrcBankReservedLessThanRequest,
        TransferReservedFailsIfSpendReservationDNE,
        TransferReservedFailsIfSpendReservationAmtIsLessThanRequest,

    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Counter for generating unique bank accounts
        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        /// Counter for transfers to the OnChainTreasuryID
        AssociatedNonceMap get(fn transfer_nonce_map): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) BankMapId => T::AssociatedId;

        /// Transfer info, must be referenced for every withdrawal from Org for AccountId
        pub TransferInfo get(fn transfer_info): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::AssociatedId =>
            Option<TransferInformation<Sender<T::AccountId, T::OrgId>, BalanceOf<T>, TransferState>>;

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique `OnChainTreasuryID`s (note the cryptographic hash function)
        pub BankStores get(fn bank_stores): map
            hasher(opaque_blake2_256) OnChainTreasuryID =>
            Option<BankState<T::AccountId, T::OrgId, BalanceOf<T>>>;

        /// Bank spend reservations
        pub SpendReservations get(fn spend_reservations): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::AssociatedId =>
            Option<SpendReservation<T::AccountId, BalanceOf<T>>>;

        /// Initialized upon first withdrawal request s.t. value contains
        /// amount remaining, if any
        pub Withdrawals get(fn withdrawals): double_map
            hasher(blake2_128_concat) (OnChainTreasuryID, T::AssociatedId),
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
            let new_transfer_id = Self::post_user_transfer(opener.clone(), new_treasury_id, deposit_amount)?;
            // infallible registration because treasury id was just generated and is unique, qed
            Self::register_org_account(new_treasury_id, org, deposit_amount, controller.clone());
            // event emission
            Self::deposit_event(RawEvent::AccountOpensOrgBankAccount(opener, new_treasury_id, new_transfer_id.sub_id, deposit_amount, org, controller));
            Ok(())
        }
        // account to existing org transfer
        #[weight = 0]
        fn account_to_org_transfer(origin, bank_id: OnChainTreasuryID, transfer_amount: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // execute transfer and return new transfer identifier
            let new_transfer_id = Self::post_user_transfer(sender.clone(), bank_id, transfer_amount)?;
            // event emission
            Self::deposit_event(RawEvent::AccountToOrgTransfer(new_transfer_id.sub_id, sender, bank_id, transfer_amount));
            Ok(())
        }
        // withdrawal by account id from org
        // #[weight = 0]
        // fn withdraw_from_org_to_account(origin, bank_id: OnChainTreasuryID, transfer_id: T::AssociatedId, amount_requested: Option<BalanceOf<T>>) -> DispatchResult {
        //     let withdrawer = ensure_signed(origin)?;
        //     let bank_account = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::WithdrawalFromOrgToAccountRequiresExistingOrgBankAccount)?;
        //     let transfer_info = <TransferInfo<T>>::get(bank_id, transfer_id).ok_or(Error::<T>::WithdrawalFromOrgToAccountRequiresValidTransferReference)?;
        //     // check if the account has withdrawn from this path before and, if so, how much is left
        //     let amount_left_for_account_to_withdraw = if let Some(left_amt) = <Withdrawals<T>>::get((bank_id, transfer_id), &withdrawer) {
        //         // => this call has been made before, here is the amount unclaimed but
        //         // this value should be updated using the same method in the other branch...
        //         left_amt
        //     } else {
        //         // should be done with the amount_left but should be done as a check
        //         Self::calculate_proportional_amount_for_account(transfer_info.amount_transferred(), withdrawer.clone(), bank_account.org())?
        //     };
        //     // amount to claim
        //     let (amount_to_claim, amount_left_after_withdrawal)  = if let Some(requested_amt) = amount_requested {
        //         ensure!(requested_amt >= amount_left_for_account_to_withdraw, Error::<T>::RequestExceedsAmountThatCanBeWithdrawnForAccountFromOrg);
        //         let amount_left_after_withdrawal = requested_amt - amount_left_for_account_to_withdraw;
        //         (requested_amt, amount_left_after_withdrawal)
        //     } else {
        //         (amount_left_for_account_to_withdraw, BalanceOf::<T>::zero())
        //     };
        //     // change transfer_info to claim
        //     let new_transfer_info = transfer_info.claim_amount(amount_to_claim).ok_or(Error::<T>::WithdrawalRequestExceedsUnClaimedTransferAmount)?;
        //     // change bank to spend from free
        //     let new_bank = bank_account.spend_from_free(amount_to_claim).ok_or(Error::<T>::WithdrawalRequestExceedsFreeOrgBankFunds)?;
        //     // make the transfer
        //     T::Currency::transfer(&withdrawer, &Self::account_id(bank_id), amount_to_claim, ExistenceRequirement::KeepAlive)?;
        //     // insert updated state
        //     <BankStores<T>>::insert(bank_id, new_bank);
        //     <TransferInfo<T>>::insert(bank_id, transfer_id, new_transfer_info);
        //     <Withdrawals<T>>::insert((bank_id, transfer_id), withdrawer.clone(), amount_left_after_withdrawal);
        //     Self::deposit_event(RawEvent::WithdrawalFromOrgToAccount(withdrawer, bank_id, transfer_id, amount_to_claim));
        //     Ok(())
        // }
    }
}

impl<T: Trait> Module<T> {
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    pub fn is_bank(id: OnChainTreasuryID) -> bool {
        !Self::id_is_available(id)
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

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, T::AssociatedId)>
    for Module<T>
{
    fn id_is_available(id: (OnChainTreasuryID, T::AssociatedId)) -> bool {
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

impl<T: Trait>
    SeededGenerateUniqueID<T::AssociatedId, (OnChainTreasuryID, BankMapId)>
    for Module<T>
{
    fn seeded_generate_unique_id(
        seed: (OnChainTreasuryID, BankMapId),
    ) -> T::AssociatedId {
        let mut id_nonce =
            <AssociatedNonceMap<T>>::get(seed.0, seed.1) + 1u32.into();
        while !Self::id_is_available((seed.0, id_nonce)) {
            id_nonce = id_nonce + 1u32.into();
        }
        <AssociatedNonceMap<T>>::insert(seed.0, seed.1, id_nonce);
        id_nonce
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
        let new_bank = BankState::new(org, deposit_amount, controller);
        // insert new bank object
        <BankStores<T>>::insert(bank_id, new_bank);
    }
}

impl<T: Trait> PostUserTransfer<OnChainTreasuryID, T::AccountId, BalanceOf<T>>
    for Module<T>
{
    type TransferId = FullBankId<T::AssociatedId>;
    // account to org transfer structure
    fn post_user_transfer(
        sender: T::AccountId,
        bank_id: OnChainTreasuryID,
        amt: BalanceOf<T>,
    ) -> Result<FullBankId<T::AssociatedId>, DispatchError> {
        ensure!(
            amt >= T::MinimumTransfer::get(),
            Error::<T>::TransferMustExceedModuleMinimum
        );
        T::Currency::transfer(
            &sender,
            &Self::account_id(bank_id),
            amt,
            ExistenceRequirement::KeepAlive,
        )?;
        let new_transfer: TransferInformation<
            Sender<T::AccountId, T::OrgId>,
            BalanceOf<T>,
            TransferState,
        > = TransferInformation::new(Sender::Account(sender), amt);
        // generate unique transfer id
        let new_transfer_id =
            Self::seeded_generate_unique_id((bank_id, BankMapId::Transfer));
        // insert new transfer
        <TransferInfo<T>>::insert(bank_id, new_transfer_id, new_transfer);
        Ok(FullBankId::new(bank_id, new_transfer_id))
    }
}

impl<T: Trait>
    ReserveOrgSpend<FullBankId<T::AssociatedId>, T::AccountId, BalanceOf<T>>
    for Module<T>
{
    fn reserve_org_spend(
        transfer_id: FullBankId<T::AssociatedId>,
        recipient: T::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<FullBankId<T::AssociatedId>, DispatchError> {
        let bank = <BankStores<T>>::get(transfer_id.id)
            .ok_or(Error::<T>::CannotReserveOrgSpendIfBankStoreDNE)?;
        let transfer =
            <TransferInfo<T>>::get(transfer_id.id, transfer_id.sub_id)
                .ok_or(Error::<T>::CannotReserveOrgSpendIfTransferDNE)?;
        // form new/updated storage objects
        let new_transfer = transfer
            .spend(amt)
            .ok_or(Error::<T>::ReserveOrgSpendExceedsFreeTransferCapital)?;
        let new_bank = bank.add_reserved(amt);
        let new_spend_reservation =
            SpendReservation::new(recipient, amt, BalanceOf::<T>::zero());
        // update storage items
        let new_reservation_id = Self::seeded_generate_unique_id((
            transfer_id.id,
            BankMapId::ReserveSpend,
        ));
        <SpendReservations<T>>::insert(
            transfer_id.id,
            new_reservation_id,
            new_spend_reservation,
        );
        <TransferInfo<T>>::insert(
            transfer_id.id,
            transfer_id.sub_id,
            new_transfer,
        );
        <BankStores<T>>::insert(transfer_id.id, new_bank);
        Ok(FullBankId::new(transfer_id.id, new_reservation_id))
    }
    fn unreserve_org_spend(
        reservation_id: FullBankId<T::AssociatedId>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let spend_reservation = <SpendReservations<T>>::get(
            reservation_id.id,
            reservation_id.sub_id,
        )
        .ok_or(Error::<T>::CannotUnreserveSpendReservationThatDNE)?;
        // these funds are removed from reserves but they DO NOT go back to the previous transfer object (that would introduce race conditions related to the unknown .state() of the transfer object)
        // -> instead, it is removed from spend reserves
        // --> in the future, I'd like these funds to be available for liquidating shares by _burning_ them
        let amount_unreserved = spend_reservation.amount_left();
        let new_bank = <BankStores<T>>::get(reservation_id.id)
            .ok_or(Error::<T>::CannotUnreserveSpendReservationIfBankStoreDNE)?
            .subtract_reserved(amount_unreserved)
            .ok_or(Error::<T>::CannotUnreserveSpendReservationIfBankReservedIsLessThanAmtToUnreserve)?;
        <SpendReservations<T>>::remove(
            reservation_id.id,
            reservation_id.sub_id,
        );
        <BankStores<T>>::insert(reservation_id.id, new_bank);
        Ok(amount_unreserved)
    }
}

impl<T: Trait>
    PostOrgTransfer<
        FullBankId<T::AssociatedId>,
        OnChainTreasuryID,
        T::AccountId,
        BalanceOf<T>,
    > for Module<T>
{
    fn direct_transfer_to_org(
        transfer_id: FullBankId<T::AssociatedId>,
        dest_bank_id: OnChainTreasuryID,
        amt: BalanceOf<T>,
    ) -> Result<FullBankId<T::AssociatedId>, DispatchError> {
        // check for safety, it is too confusing of a user error to debug if we remove this check
        ensure!(
            Self::is_bank(dest_bank_id),
            Error::<T>::TransferFailsIfDestBankDNE
        );
        // this reassignment improves readability && nothing else
        let (src_bank_id, transfer_sub_identifier) =
            (transfer_id.id, transfer_id.sub_id);
        let src_bank = <BankStores<T>>::get(src_bank_id).ok_or(
            Error::<T>::CannotTransferFromOrgToOrgIfBankForTransferReferenceDNE,
        )?;
        let updated_transfer_info = <TransferInfo<T>>::get(src_bank_id, transfer_sub_identifier)
            .ok_or(Error::<T>::CannotTransferFromOrgToOrgIfReferenceTransferDNE)?
            .spend(amt)
            .ok_or(Error::<T>::CannotTransferFromOrgToOrgIfInWrongStateOrNotEnoughFunds)?;
        // execute the transfer
        T::Currency::transfer(
            &Self::account_id(src_bank_id),
            &Self::account_id(dest_bank_id),
            amt,
            ExistenceRequirement::KeepAlive,
        )?;
        // form new org to org transfer
        let new_transfer: TransferInformation<
            Sender<T::AccountId, T::OrgId>,
            BalanceOf<T>,
            TransferState,
        > = TransferInformation::new(Sender::Org(src_bank.org()), amt);
        // generate unique transfer id
        let new_transfer_id = Self::seeded_generate_unique_id((
            dest_bank_id,
            BankMapId::Transfer,
        ));
        // update referenced src transfer
        <TransferInfo<T>>::insert(
            src_bank_id,
            transfer_sub_identifier,
            updated_transfer_info,
        );
        // insert new transfer
        <TransferInfo<T>>::insert(dest_bank_id, new_transfer_id, new_transfer);
        // return new transfer
        Ok(FullBankId::new(dest_bank_id, new_transfer_id))
    }
    fn direct_transfer_to_account(
        transfer_id: FullBankId<T::AssociatedId>,
        dest_acc: T::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        // this reassignment improves readability && nothing else
        let (src_bank_id, transfer_sub_identifier) =
            (transfer_id.id, transfer_id.sub_id);
        let src_bank = <BankStores<T>>::get(src_bank_id).ok_or(
            Error::<T>::CannotTransferFromOrgToAccountIfBankForTransferReferenceDNE,
        )?;
        let updated_transfer_info = <TransferInfo<T>>::get(src_bank_id, transfer_sub_identifier)
            .ok_or(Error::<T>::CannotTransferFromOrgToAccountIfReferenceTransferDNE)?
            .spend(amt)
            .ok_or(Error::<T>::CannotTransferFromOrgToAccountIfInWrongStateOrNotEnoughFunds)?;
        // execute the transfer
        T::Currency::transfer(
            &Self::account_id(src_bank_id),
            &dest_acc,
            amt,
            ExistenceRequirement::KeepAlive,
        )?;
        // update referenced src transfer
        <TransferInfo<T>>::insert(
            src_bank_id,
            transfer_sub_identifier,
            updated_transfer_info,
        );
        Ok(())
    }
    fn transfer_reserved_spend_to_org(
        reservation_id: FullBankId<T::AssociatedId>,
        dest_bank_id: OnChainTreasuryID,
        amt: BalanceOf<T>,
    ) -> Result<FullBankId<T::AssociatedId>, DispatchError> {
        // check for safety, it is too confusing of a user error to debug if we remove this check
        ensure!(
            Self::is_bank(dest_bank_id),
            Error::<T>::TransferFailsIfDestBankDNE
        );
        let (src_bank_id, reservation_sub_identifier) =
            (reservation_id.id, reservation_id.sub_id);
        let src_bank = <BankStores<T>>::get(src_bank_id)
            .ok_or(Error::<T>::TransferReservedFailsIfSrcBankDNE)?
            .subtract_reserved(amt)
            .ok_or(Error::<T>::TransferReservedFailsIfSrcBankReservedLessThanRequest)?;
        let updated_spend_reservation = <SpendReservations<T>>::get(src_bank_id, reservation_sub_identifier)
            .ok_or(Error::<T>::TransferReservedFailsIfSpendReservationDNE)?
            .spend(amt)
            .ok_or(Error::<T>::TransferReservedFailsIfSpendReservationAmtIsLessThanRequest)?;
        // execute transfer
        T::Currency::transfer(
            &Self::account_id(src_bank_id),
            &Self::account_id(dest_bank_id),
            amt,
            ExistenceRequirement::KeepAlive,
        )?;
        // form new transfer object
        let new_transfer: TransferInformation<
            Sender<T::AccountId, T::OrgId>,
            BalanceOf<T>,
            TransferState,
        > = TransferInformation::new(Sender::Org(src_bank.org()), amt);
        // generate unique transfer identifier
        let new_transfer_id = Self::seeded_generate_unique_id((
            dest_bank_id,
            BankMapId::Transfer,
        ));
        // insert new transfer
        <TransferInfo<T>>::insert(dest_bank_id, new_transfer_id, new_transfer);
        // update src bank storage item
        <BankStores<T>>::insert(src_bank_id, src_bank);
        // update src reservation
        <SpendReservations<T>>::insert(
            src_bank_id,
            reservation_sub_identifier,
            updated_spend_reservation,
        );
        Ok(FullBankId::new(dest_bank_id, new_transfer_id))
    }
    fn transfer_reserved_spend_to_account(
        reservation_id: FullBankId<T::AssociatedId>,
        dest_acc: T::AccountId,
        amt: BalanceOf<T>,
    ) -> Result<(), DispatchError> {
        let (src_bank_id, reservation_sub_identifier) =
            (reservation_id.id, reservation_id.sub_id);
        let src_bank = <BankStores<T>>::get(src_bank_id)
            .ok_or(Error::<T>::TransferReservedFailsIfSrcBankDNE)?
            .subtract_reserved(amt)
            .ok_or(Error::<T>::TransferReservedFailsIfSrcBankReservedLessThanRequest)?;
        let updated_spend_reservation = <SpendReservations<T>>::get(src_bank_id, reservation_sub_identifier)
            .ok_or(Error::<T>::TransferReservedFailsIfSpendReservationDNE)?
            .spend(amt)
            .ok_or(Error::<T>::TransferReservedFailsIfSpendReservationAmtIsLessThanRequest)?;
        // execute transfer
        T::Currency::transfer(
            &Self::account_id(src_bank_id),
            &dest_acc,
            amt,
            ExistenceRequirement::KeepAlive,
        )?;
        // insert updated storage items
        <BankStores<T>>::insert(src_bank_id, src_bank);
        <SpendReservations<T>>::insert(
            src_bank_id,
            reservation_sub_identifier,
            updated_spend_reservation,
        );
        Ok(())
    }
}

impl<T: Trait> StopSpendsStartWithdrawals<FullBankId<T::AssociatedId>>
    for Module<T>
{
    // this method changes the state of the transfer object such that spends and reservations are no longer allowed
    // and withdrawals by members can commence (with limits based on ownership rights)
    fn stop_spends_start_withdrawals(
        transfer_id: FullBankId<T::AssociatedId>,
    ) -> Result<(), DispatchError> {
        Ok(())
    }
}
impl<T: Trait>
    WithdrawFromOrg<FullBankId<T::AssociatedId>, T::AccountId, BalanceOf<T>>
    for Module<T>
{
    fn claim_due_amount(
        transfer_id: FullBankId<T::AssociatedId>,
        for_acc: T::AccountId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        todo!()
    }
}
