#![recursion_limit = "256"]
#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This module defines rules for withdrawal from a joint account; enables
//! registered organizations to manage these accounts with share permission groups.

#[cfg(test)]
mod tests;

use util::{
    bank::{
        BankMapID, BankState, DepositInfo, InternalTransferInfo, OnChainTreasuryID,
        ReservationInfo, WithdrawalPermissions,
    },
    traits::{
        CalculateOwnership, CheckBankBalances, CommitAndTransfer, CommitSpendReservation,
        DefaultBankPermissions, DepositIntoBank, DepositSpendOps, DepositsAndSpends, ExecuteSpends,
        FreeToReserved, GenerateUniqueID, GetGroupSize, GroupMembership, IDIsAvailable, Increment,
        MoveFundsOutCommittedOnly, MoveFundsOutUnCommittedOnly, OnChainBank,
        OrganizationSupervisorPermissions, RegisterAccount, ReservationMachine,
        SeededGenerateUniqueID, ShareInformation, ShareIssuance, TermSheetExit,
    }, // if sudo, import ChainSudoPermissions
};

use codec::Codec;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    storage::IterableStorageDoubleMap,
    traits::{Currency, ExistenceRequirement, Get},
    Parameter,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AccountIdConversion, AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero}, // CheckedAdd, CheckedSub
    DispatchError,
    DispatchResult,
    Permill,
};
use sp_std::{fmt::Debug, prelude::*};

/// The balances type for this module
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: system::Trait + org::Trait + vote::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type BankAssociatedId: Parameter
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

    /// The minimum amount for opening a bank account in the context of this module
    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
        <T as Trait>::BankAssociatedId,
        Balance = BalanceOf<T>,
    {
        RegisteredNewOnChainBank(AccountId, OnChainTreasuryID, Balance, OrgId, WithdrawalPermissions<OrgId, AccountId>),
        CapitalDepositedIntoOnChainBankAccount(AccountId, OnChainTreasuryID, Balance, IpfsReference),
        SpendReservedForBankAccount(AccountId, OnChainTreasuryID, BankAssociatedId, IpfsReference, Balance, WithdrawalPermissions<OrgId, AccountId>),
        CommitSpendBeforeInternalTransfer(AccountId, OnChainTreasuryID, BankAssociatedId, Balance),
        UnReserveUncommittedReservationToMakeFree(AccountId, OnChainTreasuryID, BankAssociatedId, Balance),
        UnReserveCommittedReservationToMakeFree(AccountId, OnChainTreasuryID, BankAssociatedId, Balance),
        InternalTransferExecutedAndSpendingPowerDoledOutToController(AccountId, OnChainTreasuryID, IpfsReference, BankAssociatedId, Balance, WithdrawalPermissions<OrgId, AccountId>),
        SpendRequestForInternalTransferApprovedAndExecuted(OnChainTreasuryID, AccountId, Balance, BankAssociatedId),
        AccountLeftMembershipAndWithdrewProportionOfFreeCapitalInBank(OnChainTreasuryID, AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        BankAccountNotFoundForTermSheetExit,
        BankAccountNotFoundForDeposit,
        BankAccountNotFoundForWithdrawal,
        BankAccountNotFoundForSpendReservation,
        BankAccountNotFoundForInternalTransfer,
        SpendReservationNotFound,
        NotEnoughFundsInReservedToAllowSpend,
        CannotCommitReservedSpendBecauseAmountExceedsSpendReservation,
        NotEnoughFundsInFreeToAllowSpend,
        NotEnoughFundsInFreeToAllowReservation,
        AllSpendsFromReserveMustReferenceInternalTransferNotFound,
        NotEnoughFundsCommittedToSatisfyUnreserveAndFreeRequest,
        NotEnoughFundsCommittedToEnableInternalTransfer,
        NotEnoughFundsInSpendReservationUnCommittedToSatisfyUnreserveUnCommittedRequest,
        NotEnoughFundsInBankReservedToSatisfyUnReserveUnComittedRequest,
        RegistrationMustDepositAboveModuleMinimum,
        AccountMatchesNoneOfTwoControllers,
        AccountHasNoWeightedOwnershipInOrg,
        BankAccountNotFoundForCommittingAndTransferringInOneStep,
        BankAccountNotFoundForUnReservingCommitted,
        BankAccountNotFoundForUnReservingUnCommitted,
        BankAccountNotFoundForSpendCommitment,
        CannotRegisterBankAccountBCPermissionsCheckFails,
        CannotReserveSpendBCPermissionsCheckFails,
        CannotCommitSpendBCPermissionsCheckFails,
        CannotUnReserveUncommittedBCPermissionsCheckFails,
        CannotUnReserveCommittedBCPermissionsCheckFails,
        CannotTransferSpendingPowerBCPermissionsCheckFails,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Counter for generating uniqe bank accounts
        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        /// Map for managing counters associated with associated maps
        BankAssociatedNonces get(fn bank_associated_nonces): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) BankMapID => T::BankAssociatedId;

        /// Source of truth for OnChainTreasuryId uniqueness checks
        /// WARNING: do not append a prefix because the keyspace is used directly for checking uniqueness of `OnChainTreasuryId`
        /// TODO: pre-reserve any known ModuleId's that could be accidentally generated that already exist elsewhere
        pub BankStores get(fn bank_stores): map
            hasher(blake2_128_concat) OnChainTreasuryID => Option<BankState<T::OrgId, WithdrawalPermissions<T::OrgId, T::AccountId>, BalanceOf<T>>>;

        /// All deposits made into the joint bank account represented by OnChainTreasuryID
        pub Deposits get(fn deposits): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::BankAssociatedId => Option<DepositInfo<T::AccountId, T::IpfsReference, BalanceOf<T>>>;

        /// Spend reservations which designated a committee for formally transferring ownership to specific destination addresses
        pub SpendReservations get(fn spend_reservations): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::BankAssociatedId => Option<ReservationInfo<T::IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::OrgId, T::AccountId>>>;

        /// Internal transfers of control over capital that allow transfer liquidity rights to the current controller
        pub InternalTransfers get(fn internal_transfers): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::BankAssociatedId => Option<InternalTransferInfo<T::BankAssociatedId, T::IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::OrgId, T::AccountId>>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn deposit_from_signer_for_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            amount: BalanceOf<T>,
            //savings_tax: Option<Permill>, // l8rrr
            reason: T::IpfsReference,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;

            Self::deposit_into_bank(depositer.clone(), bank_id, amount, reason.clone())?;
            Self::deposit_event(RawEvent::CapitalDepositedIntoOnChainBankAccount(depositer, bank_id, amount, reason));
            Ok(())
        }
        #[weight = 0]
        fn register_and_seed_for_bank_account(
            origin,
            seed: BalanceOf<T>,
            hosting_org: T::OrgId, // pre-requisite is registered organization
            bank_controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
        ) -> DispatchResult {
            let seeder = ensure_signed(origin)?;
            let authentication = Self::can_register_account(seeder.clone(), hosting_org) && Self::withdrawal_permissions_satisfy_org_standards(hosting_org, bank_controller.clone());
            ensure!(authentication, Error::<T>::CannotRegisterBankAccountBCPermissionsCheckFails);
            let new_bank_id = Self::register_account(hosting_org, seeder.clone(), seed, bank_controller.clone())?;
            Self::deposit_event(RawEvent::RegisteredNewOnChainBank(seeder, new_bank_id, seed, hosting_org, bank_controller));
            Ok(())
        }
        #[weight = 0]
        fn reserve_spend_for_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            reason: T::IpfsReference,
            amount: BalanceOf<T>,
            controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
        ) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            let authentication = Self::can_reserve_for_spend(reserver.clone(), bank_id)?;
            ensure!(authentication, Error::<T>::CannotReserveSpendBCPermissionsCheckFails);
            let new_reservation_id = Self::reserve_for_spend(bank_id, reason.clone(), amount, controller.clone())?;
            Self::deposit_event(RawEvent::SpendReservedForBankAccount(reserver, bank_id, new_reservation_id, reason, amount, controller));
            Ok(())
        }
        #[weight = 0]
        fn commit_reserved_spend_for_transfer_inside_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: T::BankAssociatedId,
            reason: T::IpfsReference,
            amount: BalanceOf<T>,
            future_controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
        ) -> DispatchResult {
            let committer = ensure_signed(origin)?;
            let authentication = Self::can_commit_reserved_spend_for_transfer(committer.clone(), bank_id)?;
            ensure!(authentication, Error::<T>::CannotCommitSpendBCPermissionsCheckFails);
            Self::commit_reserved_spend_for_transfer(bank_id, reservation_id, amount)?;
            Self::deposit_event(RawEvent::CommitSpendBeforeInternalTransfer(committer, bank_id, reservation_id, amount));
            Ok(())
        }
        #[weight = 0]
        fn unreserve_uncommitted_reservation_to_make_free(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: T::BankAssociatedId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let qualified_bank_controller = ensure_signed(origin)?;
            let authentication = Self::can_unreserve_uncommitted_to_make_free(qualified_bank_controller.clone(), bank_id)?;
            ensure!(authentication, Error::<T>::CannotUnReserveUncommittedBCPermissionsCheckFails);
            Self::unreserve_uncommitted_to_make_free(bank_id, reservation_id, amount)?;
            Self::deposit_event(RawEvent::UnReserveUncommittedReservationToMakeFree(qualified_bank_controller, bank_id, reservation_id, amount));
            Ok(())
        }
        #[weight = 0]
        fn unreserve_committed_reservation_to_make_free(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: T::BankAssociatedId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let qualified_spend_reservation_controller = ensure_signed(origin)?;
            let authentication = Self::can_unreserve_committed_to_make_free(qualified_spend_reservation_controller.clone(), bank_id)?;
            ensure!(authentication, Error::<T>::CannotUnReserveCommittedBCPermissionsCheckFails);
            Self::unreserve_committed_to_make_free(bank_id, reservation_id, amount)?;
            Self::deposit_event(RawEvent::UnReserveCommittedReservationToMakeFree(qualified_spend_reservation_controller, bank_id, reservation_id, amount));
            Ok(())
        }
        #[weight = 0]
        fn transfer_spending_for_spend_commitment(
            origin,
            bank_id: OnChainTreasuryID,
            reason: T::IpfsReference,
            reservation_id: T::BankAssociatedId,
            amount: BalanceOf<T>,
            committed_controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
        ) -> DispatchResult {
            let qualified_spend_reservation_controller = ensure_signed(origin)?;
            let authentication = Self::can_transfer_spending_power(qualified_spend_reservation_controller.clone(), bank_id)?;
            ensure!(authentication, Error::<T>::CannotTransferSpendingPowerBCPermissionsCheckFails);
            Self::transfer_spending_power(bank_id, reason.clone(), reservation_id, amount, committed_controller.clone())?;
            Self::deposit_event(RawEvent::InternalTransferExecutedAndSpendingPowerDoledOutToController(qualified_spend_reservation_controller, bank_id, reason, reservation_id, amount, committed_controller));
            Ok(())
        }
        #[weight = 0]
        fn withdraw_by_referencing_internal_transfer(
            origin,
            bank_id: OnChainTreasuryID,
            transfer_id: T::BankAssociatedId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let requester = ensure_signed(origin)?;
            let amount_received = Self::spend_from_transfers(
                bank_id,
                transfer_id,
                requester.clone(),
                amount,
            )?;
            Self::deposit_event(RawEvent::SpendRequestForInternalTransferApprovedAndExecuted(bank_id, requester, amount_received, transfer_id));
            Ok(())
        }
        #[weight = 0]
        fn burn_all_shares_to_leave_weighted_membership_bank_and_withdraw_related_free_capital(
            origin,
            bank_id: OnChainTreasuryID,
        ) -> DispatchResult {
            let leaving_member = ensure_signed(origin)?;
            let amount_withdrawn_by_burning_shares = Self::burn_shares_to_exit_bank_ownership(leaving_member.clone(), bank_id)?;
            Self::deposit_event(RawEvent::AccountLeftMembershipAndWithdrewProportionOfFreeCapitalInBank(bank_id, leaving_member, amount_withdrawn_by_burning_shares));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    // deposits
    pub fn get_deposits_by_account(
        bank_id: OnChainTreasuryID,
        depositer: T::AccountId,
    ) -> Option<Vec<DepositInfo<T::AccountId, T::IpfsReference, BalanceOf<T>>>> {
        let depositers_deposits = <Deposits<T>>::iter()
            .filter(|(id, _, deposit)| id == &bank_id && deposit.depositer() == depositer)
            .map(|(_, _, deposit)| deposit)
            .collect::<Vec<DepositInfo<T::AccountId, T::IpfsReference, BalanceOf<T>>>>();
        if depositers_deposits.is_empty() {
            None
        } else {
            Some(depositers_deposits)
        }
    }
    pub fn total_capital_deposited_by_account(
        bank_id: OnChainTreasuryID,
        depositer: T::AccountId,
    ) -> BalanceOf<T> {
        <Deposits<T>>::iter()
            .filter(|(id, _, deposit)| id == &bank_id && deposit.depositer() == depositer)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, deposit)| {
                acc + deposit.amount()
            })
    }
    // reservations
    pub fn get_amount_left_in_spend_reservation(
        bank_id: OnChainTreasuryID,
        reservation_id: T::BankAssociatedId,
    ) -> Option<BalanceOf<T>> {
        if let Some(spend_reservation) = <SpendReservations<T>>::get(bank_id, reservation_id) {
            Some(spend_reservation.amount())
        } else {
            None
        }
    }
    pub fn get_reservations_for_governance_config(
        bank_id: OnChainTreasuryID,
        invoker: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Option<
        Vec<
            ReservationInfo<
                T::IpfsReference,
                BalanceOf<T>,
                WithdrawalPermissions<T::OrgId, T::AccountId>,
            >,
        >,
    > {
        let ret = <SpendReservations<T>>::iter()
            .filter(|(id, _, reservation)| id == &bank_id && reservation.controller() == invoker)
            .map(|(_, _, reservation)| reservation)
            .collect::<Vec<
                ReservationInfo<
                    T::IpfsReference,
                    BalanceOf<T>,
                    WithdrawalPermissions<T::OrgId, T::AccountId>,
                >,
            >>();
        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }
    pub fn total_capital_reserved_for_governance_config(
        bank_id: OnChainTreasuryID,
        invoker: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> BalanceOf<T> {
        <SpendReservations<T>>::iter()
            .filter(|(id, _, reservation)| id == &bank_id && reservation.controller() == invoker)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, reservation)| {
                acc + reservation.amount()
            })
    }
    // transfers
    pub fn get_amount_left_in_approved_transfer(
        bank_id: OnChainTreasuryID,
        transfer_id: T::BankAssociatedId,
    ) -> Option<BalanceOf<T>> {
        if let Some(internal_transfer) = <InternalTransfers<T>>::get(bank_id, transfer_id) {
            Some(internal_transfer.amount())
        } else {
            None
        }
    }
    pub fn get_transfers_for_governance_config(
        bank_id: OnChainTreasuryID,
        invoker: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Option<
        Vec<
            InternalTransferInfo<
                T::BankAssociatedId,
                T::IpfsReference,
                BalanceOf<T>,
                WithdrawalPermissions<T::OrgId, T::AccountId>,
            >,
        >,
    > {
        let ret = <InternalTransfers<T>>::iter()
            .filter(|(id, _, transfer)| id == &bank_id && transfer.controller() == invoker)
            .map(|(_, _, transfer)| transfer)
            .collect::<Vec<
                InternalTransferInfo<
                    T::BankAssociatedId,
                    T::IpfsReference,
                    BalanceOf<T>,
                    WithdrawalPermissions<T::OrgId, T::AccountId>,
                >,
            >>();
        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }
    pub fn total_capital_transferred_to_governance_config(
        bank_id: OnChainTreasuryID,
        invoker: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> BalanceOf<T> {
        <InternalTransfers<T>>::iter()
            .filter(|(id, _, transfer)| id == &bank_id && transfer.controller() == invoker)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, transfer)| {
                acc + transfer.amount()
            })
    }
}

impl<T: Trait> IDIsAvailable<OnChainTreasuryID> for Module<T> {
    fn id_is_available(id: OnChainTreasuryID) -> bool {
        <BankStores<T>>::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, BankMapID, T::BankAssociatedId)> for Module<T> {
    fn id_is_available(id: (OnChainTreasuryID, BankMapID, T::BankAssociatedId)) -> bool {
        match id.1 {
            BankMapID::Deposit => <Deposits<T>>::get(id.0, id.2).is_none(),
            BankMapID::Reservation => <SpendReservations<T>>::get(id.0, id.2).is_none(),
            BankMapID::InternalTransfer => <InternalTransfers<T>>::get(id.0, id.2).is_none(),
        }
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::BankAssociatedId, (OnChainTreasuryID, BankMapID)>
    for Module<T>
{
    fn seeded_generate_unique_id(seed: (OnChainTreasuryID, BankMapID)) -> T::BankAssociatedId {
        let mut new_id = <BankAssociatedNonces<T>>::get(seed.0, seed.1) + 1u32.into();
        while !Self::id_is_available((seed.0, seed.1, new_id)) {
            new_id += 1u32.into();
        }
        <BankAssociatedNonces<T>>::insert(seed.0, seed.1, new_id);
        new_id
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

impl<T: Trait> OnChainBank for Module<T> {
    type TreasuryId = OnChainTreasuryID;
    type AssociatedId = T::BankAssociatedId;
}

impl<T: Trait>
    RegisterAccount<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
    > for Module<T>
{
    fn register_account(
        owners: T::OrgId,
        from: T::AccountId,
        amount: BalanceOf<T>,
        operators: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<Self::TreasuryId, DispatchError> {
        ensure!(
            amount >= T::MinimumInitialDeposit::get(),
            Error::<T>::RegistrationMustDepositAboveModuleMinimum
        );
        // init new bank object
        let new_bank = BankState::new_from_deposit(owners, amount, operators);
        // generate bank id
        let generated_id = Self::generate_unique_id();
        let to = Self::account_id(generated_id);
        // make transfer to seed joint bank account with amount
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        // insert new bank store
        <BankStores<T>>::insert(generated_id, new_bank);
        Ok(generated_id)
    }
    fn verify_owner(bank_id: Self::TreasuryId, org: T::OrgId) -> bool {
        if let Some(account) = <BankStores<T>>::get(bank_id) {
            account.owners() == org
        } else {
            false
        }
    }
}

impl<T: Trait>
    CalculateOwnership<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
        Permill,
    > for Module<T>
{
    fn calculate_proportion_ownership_for_account(
        account: T::AccountId,
        group: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<Permill, DispatchError> {
        match group {
            WithdrawalPermissions::TwoAccounts(acc1, acc2) => {
                // assumes that we never use this with acc1 == acc2; use sudo in that situation
                if acc1 == account || acc2 == account {
                    Ok(Permill::from_percent(50))
                } else {
                    Err(Error::<T>::AccountMatchesNoneOfTwoControllers.into())
                }
            }
            WithdrawalPermissions::JointOrgAccount(org_id) => {
                let issuance = <org::Module<T>>::total_issuance(org_id);
                if issuance > T::Shares::zero() {
                    // weighted group
                    let acc_ownership = <org::Module<T>>::get_share_profile(org_id, &account)
                        .ok_or(Error::<T>::AccountHasNoWeightedOwnershipInOrg)?;
                    Ok(Permill::from_rational_approximation(
                        acc_ownership.total(),
                        issuance,
                    ))
                } else {
                    // flat group
                    let organization_size = <org::Module<T>>::get_size_of_group(org_id);
                    Ok(Permill::from_rational_approximation(1, organization_size))
                }
            }
        }
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let proportion_due = Self::calculate_proportion_ownership_for_account(account, group)?;
        Ok(proportion_due * amount)
    }
}

impl<T: Trait> DepositsAndSpends<BalanceOf<T>> for Module<T> {
    type Bank = BankState<T::OrgId, WithdrawalPermissions<T::OrgId, T::AccountId>, BalanceOf<T>>;
    fn make_infallible_deposit_into_free(bank: Self::Bank, amount: BalanceOf<T>) -> Self::Bank {
        bank.deposit_into_free(amount)
    }
    fn fallible_spend_from_reserved(
        bank: Self::Bank,
        amount: BalanceOf<T>,
    ) -> Result<Self::Bank, DispatchError> {
        let new_bank = bank
            .spend_from_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsInReservedToAllowSpend)?;
        Ok(new_bank)
    }
    fn fallible_spend_from_free(
        bank: Self::Bank,
        amount: BalanceOf<T>,
    ) -> Result<Self::Bank, DispatchError> {
        let new_bank = bank
            .spend_from_free(amount)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToAllowSpend)?;
        Ok(new_bank)
    }
}

impl<T: Trait> CheckBankBalances<BalanceOf<T>> for Module<T> {
    fn get_bank_store(bank_id: Self::TreasuryId) -> Option<Self::Bank> {
        <BankStores<T>>::get(bank_id)
    }
    fn calculate_total_bank_balance_from_balances(
        bank_id: Self::TreasuryId,
    ) -> Option<BalanceOf<T>> {
        let bank_account = Self::account_id(bank_id);
        let bank_balance = T::Currency::total_balance(&bank_account);
        if bank_balance == 0.into() {
            None
        } else {
            Some(bank_balance)
        }
    }
}

// We could NOT have the extra storage lookup in here but
// the API design is much more extensible this way. See issue #85
impl<T: Trait>
    DefaultBankPermissions<
        T::OrgId,
        T::AccountId,
        BalanceOf<T>,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
    > for Module<T>
{
    fn can_register_account(account: T::AccountId, on_behalf_of: T::OrgId) -> bool {
        // only the organization supervisor can register a bank account
        <org::Module<T>>::is_organization_supervisor(on_behalf_of, &account)
    }
    fn withdrawal_permissions_satisfy_org_standards(
        _org: T::OrgId,
        _withdrawal_permissions: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> bool {
        // an example might require that withdrawal_permissions is a subgroup
        //or contains members of OrgId but I don't think that's necessary to
        //impl as a default...sometimes no default is OK
        true
    }
    // bank.operators() can make spend reservations (indicates funding intent by beginning the formal shift of capital control)
    fn can_reserve_for_spend(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account =
            <BankStores<T>>::get(bank).ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let ret_bool = match bank_account.operators() {
            WithdrawalPermissions::TwoAccounts(acc1, acc2) => acc1 == account || acc2 == account,
            WithdrawalPermissions::JointOrgAccount(org_id) => {
                <org::Module<T>>::is_member_of_group(org_id, &account)
            }
        };
        Ok(ret_bool)
    }
    // bank.operators() can make spend commitments, thereby putting these funds outside of bank.owners() immediate reach
    fn can_commit_reserved_spend_for_transfer(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account =
            <BankStores<T>>::get(bank).ok_or(Error::<T>::BankAccountNotFoundForSpendCommitment)?;
        let ret_bool = match bank_account.operators() {
            WithdrawalPermissions::TwoAccounts(acc1, acc2) => acc1 == account || acc2 == account,
            WithdrawalPermissions::JointOrgAccount(org_id) => {
                <org::Module<T>>::is_member_of_group(org_id, &account)
            }
        };
        Ok(ret_bool)
    }
    // bank.owners() || bank.operators() can unreserve if not committed
    fn can_unreserve_uncommitted_to_make_free(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::BankAccountNotFoundForUnReservingUnCommitted)?;
        let ret_bool = <org::Module<T>>::is_member_of_group(bank_account.owners(), &account)
            || match bank_account.operators() {
                WithdrawalPermissions::TwoAccounts(acc1, acc2) => {
                    acc1 == account || acc2 == account
                }
                WithdrawalPermissions::JointOrgAccount(org_id) => {
                    <org::Module<T>>::is_member_of_group(org_id, &account)
                }
            };
        Ok(ret_bool)
    }
    // ONLY bank.operators() can unreserve committed funds
    fn can_unreserve_committed_to_make_free(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::BankAccountNotFoundForUnReservingCommitted)?;
        let ret_bool = match bank_account.operators() {
            WithdrawalPermissions::TwoAccounts(acc1, acc2) => acc1 == account || acc2 == account,
            WithdrawalPermissions::JointOrgAccount(org_id) => {
                <org::Module<T>>::is_member_of_group(org_id, &account)
            }
        };
        Ok(ret_bool)
    }
    // bank.owners() || bank.operators() can transfer spending power (through reference of existing spend commitment)
    fn can_transfer_spending_power(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::BankAccountNotFoundForUnReservingUnCommitted)?;
        let ret_bool = <org::Module<T>>::is_member_of_group(bank_account.owners(), &account)
            || match bank_account.operators() {
                WithdrawalPermissions::TwoAccounts(acc1, acc2) => {
                    acc1 == account || acc2 == account
                }
                WithdrawalPermissions::JointOrgAccount(org_id) => {
                    <org::Module<T>>::is_member_of_group(org_id, &account)
                }
            };
        Ok(ret_bool)
    }
    // supervisor-oriented permissions for committing and transferring withdrawal permissions in a single step
    fn can_commit_and_transfer_spending_power(
        account: T::AccountId,
        bank: Self::TreasuryId,
    ) -> Result<bool, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::BankAccountNotFoundForCommittingAndTransferringInOneStep)?;
        Ok(
            <org::Module<T>>::is_organization_supervisor(bank_account.owners(), &account)
                || match bank_account.operators() {
                    WithdrawalPermissions::TwoAccounts(acc1, acc2) => {
                        acc1 == account || acc2 == account
                    }
                    WithdrawalPermissions::JointOrgAccount(org_id) => {
                        <org::Module<T>>::is_organization_supervisor(org_id, &account)
                    }
                },
        )
    }
}

impl<T: Trait>
    DepositIntoBank<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
        T::IpfsReference,
    > for Module<T>
{
    fn deposit_into_bank(
        from: T::AccountId,
        to_bank_id: Self::TreasuryId,
        amount: BalanceOf<T>,
        reason: T::IpfsReference,
    ) -> Result<Self::AssociatedId, DispatchError> {
        let bank_account =
            <BankStores<T>>::get(to_bank_id).ok_or(Error::<T>::BankAccountNotFoundForDeposit)?;
        // make the transfer
        let dest = Self::account_id(to_bank_id);
        T::Currency::transfer(&from, &dest, amount, ExistenceRequirement::KeepAlive)?;
        // update the amount stored in the bank
        let updated_bank_balance = Self::make_infallible_deposit_into_free(bank_account, amount);
        <BankStores<T>>::insert(to_bank_id, updated_bank_balance);
        // form the deposit, no savings_pct allocated
        let new_deposit = DepositInfo::new(from, reason, amount);
        // generate unique deposit
        let deposit_id = Self::seeded_generate_unique_id((to_bank_id, BankMapID::Deposit));

        // TODO: when will we delete this, how long is this going to stay in storage?
        <Deposits<T>>::insert(to_bank_id, deposit_id, new_deposit);
        Ok(deposit_id)
    }
}

impl<T: Trait>
    ReservationMachine<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
        T::IpfsReference,
    > for Module<T>
{
    fn reserve_for_spend(
        bank_id: Self::TreasuryId,
        reason: T::IpfsReference,
        amount: BalanceOf<T>,
        controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<Self::AssociatedId, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        // create Reservation Info object with 100 percent of it uncommitted
        let new_spend_reservation = ReservationInfo::new(reason, amount, controller);
        // change bank_account such free is less and reserved is more
        let new_bank = bank_account
            .move_from_free_to_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToAllowReservation)?;
        let reservation_id = Self::seeded_generate_unique_id((bank_id, BankMapID::Reservation));
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank);
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(reservation_id)
    }
    fn commit_reserved_spend_for_transfer(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // only commit the reserved part and return error if not enough funds
        let reservation_after_commit = spend_reservation
            .commit_spend_reservation(amount)
            .ok_or(Error::<T>::CannotCommitReservedSpendBecauseAmountExceedsSpendReservation)?;
        // insert changed spend reservation
        <SpendReservations<T>>::insert(bank_id, reservation_id, reservation_after_commit);
        Ok(())
    }
    // bank controller can unreserve if not committed
    fn unreserve_uncommitted_to_make_free(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // this request must be approved by unreserving from the spend_reservation's uncommitted funds
        let new_spend_reservation = spend_reservation
            .move_funds_out_uncommitted_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsInSpendReservationUnCommittedToSatisfyUnreserveUnCommittedRequest)?;

        // the change in the underlying bank account is equivalent to spending reserved and increasing free by the same amount
        let new_bank_account = bank_account
            .spend_from_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsInBankReservedToSatisfyUnReserveUnComittedRequest)?
            .deposit_into_free(amount);
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank_account);
        // insert update spend reservation object (with the new, lower amount reserved)
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(())
    }
    fn unreserve_committed_to_make_free(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // ensure that the amount is less than the spend reservation amount
        let new_spend_reservation = spend_reservation
            .move_funds_out_committed_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsCommittedToSatisfyUnreserveAndFreeRequest)?;
        // the change in the bank account is equivalent to spending reserved and increasing free by the same amount
        let new_bank_account = bank_account
            .spend_from_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsCommittedToSatisfyUnreserveAndFreeRequest)?
            .deposit_into_free(amount);
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank_account);
        // insert update spend reservation object (with the new, lower amount reserved)
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(())
    }
    // Allocate some funds (previously set aside for spending reasons) to be withdrawable by new group
    // - this is an internal transfer to a team and it makes this capital withdrawable by them
    fn transfer_spending_power(
        bank_id: Self::TreasuryId,
        reason: T::IpfsReference,
        // reference to specific reservation
        reservation_id: Self::AssociatedId,
        amount: BalanceOf<T>,
        // move control of funds to new outer group which can reserve or withdraw directly
        new_controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<Self::AssociatedId, DispatchError> {
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForInternalTransfer)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // fallible spend from committed part of `ReservationInfo` object
        let new_spend_reservation = spend_reservation
            .move_funds_out_committed_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsCommittedToEnableInternalTransfer)?;
        // form a transfer_info
        let new_transfer =
            InternalTransferInfo::new(reservation_id, reason, amount, new_controller.clone());
        // generate the unique transfer_id
        let new_transfer_id =
            Self::seeded_generate_unique_id((bank_id, BankMapID::InternalTransfer));
        // insert transfer_info, thereby unlocking the capital for the `new_controller` group
        <InternalTransfers<T>>::insert(bank_id, new_transfer_id, new_transfer);
        // insert update reservation info after the transfer was made
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(new_transfer_id)
    }
}

impl<T: Trait>
    CommitAndTransfer<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
        T::IpfsReference,
    > for Module<T>
{
    fn commit_and_transfer_spending_power(
        bank_id: Self::TreasuryId,
        reservation_id: Self::AssociatedId,
        reason: T::IpfsReference,
        amount: BalanceOf<T>,
        new_controller: WithdrawalPermissions<T::OrgId, T::AccountId>,
    ) -> Result<Self::AssociatedId, DispatchError> {
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForInternalTransfer)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // ensure that the amount is less than the spend reservation amount
        let new_spend_reservation = spend_reservation
            .move_funds_out_uncommitted_only(amount) // notably does not reach into committed funds!
            .ok_or(Error::<T>::NotEnoughFundsCommittedToEnableInternalTransfer)?;
        // form a transfer_info
        let new_transfer =
            InternalTransferInfo::new(reservation_id, reason, amount, new_controller);
        // generate the unique transfer_id
        let new_transfer_id =
            Self::seeded_generate_unique_id((bank_id, BankMapID::InternalTransfer));
        // insert transfer_info, thereby unlocking the capital for the `new_controller` group
        <InternalTransfers<T>>::insert(bank_id, new_transfer_id, new_transfer);
        // insert update reservation info after the transfer was made
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(new_transfer_id)
    }
}

impl<T: Trait>
    ExecuteSpends<
        T::OrgId,
        T::AccountId,
        WithdrawalPermissions<T::OrgId, T::AccountId>,
        BalanceOf<T>,
        T::IpfsReference,
    > for Module<T>
{
    /// This method authenticates the spend by checking that the caller
    /// input follows the same shape as the bank's controller...
    /// => any method that calls this one will need to define local
    /// permissions for who can form the request as well
    /// as how to constrain the validity of that request
    /// based on their ownership/permissions
    /// ==> this will be called to liquidate free capital by burning bank controller ownership
    fn spend_from_free(
        from_bank_id: Self::TreasuryId,
        to: T::AccountId,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(from_bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForWithdrawal)?;
        // update the amount stored in the bank
        let bank_after_withdrawal = Self::fallible_spend_from_free(bank_account, amount)?;
        // make the transfer
        let from = Self::account_id(from_bank_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        <BankStores<T>>::insert(from_bank_id, bank_after_withdrawal);
        Ok(())
    }
    /// Authenticates the spend within this method based on the identity of `to`
    /// in relation to the `transfer_certificate`. This is how most (almost all)
    /// withdrawals should occur
    fn spend_from_transfers(
        from_bank_id: Self::TreasuryId,
        id: Self::AssociatedId, // refers to InternalTransfer, which transfers control over a subset of the overall funds
        to: T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let bank_account = <BankStores<T>>::get(from_bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForWithdrawal)?;
        let transfer_certificate = <InternalTransfers<T>>::get(from_bank_id, id)
            .ok_or(Error::<T>::AllSpendsFromReserveMustReferenceInternalTransferNotFound)?;
        // calculate due amount
        let due_amount = Self::calculate_proportional_amount_for_account(
            transfer_certificate.amount(),
            to.clone(),
            transfer_certificate.controller(),
        )?;
        ensure!(
            due_amount >= amount,
            Error::<T>::NotEnoughFundsInReservedToAllowSpend
        );
        let new_transfer_certificate = transfer_certificate
            .move_funds_out_committed_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsInReservedToAllowSpend)?;
        // update the bank store
        let bank_after_withdrawal = Self::fallible_spend_from_reserved(bank_account, amount)?;
        // make the transfer
        let from = Self::account_id(from_bank_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        // insert updated transfer certificate after amount is spent
        <InternalTransfers<T>>::insert(from_bank_id, id, new_transfer_certificate);
        <BankStores<T>>::insert(from_bank_id, bank_after_withdrawal);
        Ok(amount)
    }
}

impl<T: Trait> TermSheetExit<T::AccountId, BalanceOf<T>> for Module<T> {
    // caller should only be rage_quitter!
    fn burn_shares_to_exit_bank_ownership(
        rage_quitter: T::AccountId,
        bank_id: Self::TreasuryId,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let bank_account =
            <BankStores<T>>::get(bank_id).ok_or(Error::<T>::BankAccountNotFoundForTermSheetExit)?;
        let withdrawal_amount = Self::calculate_proportional_amount_for_account(
            bank_account.free(),
            rage_quitter.clone(),
            WithdrawalPermissions::JointOrgAccount(bank_account.owners()),
        )?;
        // <here is where we might apply some discount based on a vesting period/schedule>
        // burn the shares first
        let _ = <org::Module<T>>::burn(bank_account.owners(), rage_quitter.clone(), None, false)?;
        // Then make the transfer
        Self::spend_from_free(bank_id, rage_quitter, withdrawal_amount)?;
        // -> TODO: if transfer errs, there should be a dispute process to recover share ownership by presenting
        // the transaction that Erred
        Ok(withdrawal_amount)
    }
}
