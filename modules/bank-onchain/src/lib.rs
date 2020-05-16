#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::{
    bank::{
        BankAssociatedIdentifiers, BankState, DepositInfo, InternalTransferInfo, OnChainTreasuryID,
        PaymentConfirmation, ReservationInfo, WithdrawalPermissions,
    },
    court::Evidence,
    organization::{FormedOrganization, ShareID},
    traits::{
        BankDepositsAndSpends, BankReservations, BankSpends, CheckBankBalances, DepositInformation,
        DepositIntoBank, DepositSpendOps, GenerateUniqueID, GetInnerOuterShareGroups,
        IDIsAvailable, OffChainBank, OnChainBank, OrgChecks, OrganizationDNS,
        OwnershipProportionCalculations, RegisterBankAccount, RegisterOffChainBankAccount,
        RegisterShareGroup, ShareGroupChecks, SupervisorPermissions, SupportedOrganizationShapes,
        TransferInformation, WeightedShareIssuanceWrapper, WeightedShareWrapper,
    },
};

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, ensure,
    storage::IterableStorageDoubleMap,
    traits::{Currency, ExistenceRequirement, Get},
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{
    traits::{AccountIdConversion, Zero},
    DispatchError, DispatchResult, Permill,
};
use sp_std::prelude::*;

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The organization identfier
pub type OrgId = u32;
/// The deposit identifier
pub type DepositId = u32;
/// The spend reservation identifier
pub type ReservationId = u32;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::Organization as WeightedShareWrapper<
    u32,
    u32,
    <T as frame_system::Trait>::AccountId,
>>::Shares;
/// The balances type for this module
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;
/// This is the governance config used for things in this prototype
/// - it reads `(OrgId, ShareID) s.t. ShareID = { Flat(u32), Weighted(u32)}` such that each refers to records in separate inherited modules in `Organization`
pub type GovernanceConfig<T> =
    (
        u32,
        <<T as Trait>::Organization as ShareGroupChecks<
            u32,
            <T as frame_system::Trait>::AccountId,
        >>::MultiShareIdentifier,
    );

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    type Organization: OrgChecks<u32, Self::AccountId>
        + ShareGroupChecks<u32, Self::AccountId>
        + GetInnerOuterShareGroups<u32, Self::AccountId>
        + SupervisorPermissions<u32, Self::AccountId>
        + WeightedShareWrapper<u32, u32, Self::AccountId>
        + WeightedShareIssuanceWrapper<u32, u32, Self::AccountId, Permill>
        + RegisterShareGroup<u32, u32, Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<u32, Self::AccountId, IpfsReference>;

    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        NewOnChainTreasuryRegisteredWithSudoPermissions(OnChainTreasuryID, AccountId),
        NewOnChainTreasuryRegisteredWithFlatShareGroupPermissions(OnChainTreasuryID, OrgId, ShareID),
        NewOnChainTreasuryRegisteredWithWeightedShareGroupPermissions(OnChainTreasuryID, OrgId, ShareID),
        CapitalDepositedIntoOnChainBankAccount(AccountId, OnChainTreasuryID, Balance, IpfsReference),
        SudoWithdrawalFromOnChainBankAccount(OnChainTreasuryID, AccountId, Balance),
        WeightedShareGroupMemberClaimedPortionOfDepositToWithdraw(OnChainTreasuryID, AccountId, Balance),
        WeightedShareGroupMemberBurnedSharesToClaimProportionalWithdrawal(OnChainTreasuryID, AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        AccountNotAuthorizedToTransferSpendingPowerForSpendReservation,
        MustHaveCertainAuthorityToRegisterBankAccount,
        MustBeWeightedShareGroupToCalculatePortionOfOnChainDeposit,
        CannotWithdrawIfOnChainBankDNE,
        CannotClaimDepositFromBankThatDNE,
        CannotCalculateDepositPortionFromBankThatDNE,
        CannotCalculateLiquidShareCapitalFromBankThatDNE,
        CannotBurnEnoughSharesToLiquidateCapitalForWithdrawalRequest,
        DepositCannotBeFoundToCalculateCorrectPortion,
        CanOnlyClaimUpToOwnershipPortionByDefault,
        BankAccountNotFoundForDeposit,
        BankAccountNotFoundForWithdrawal,
        BankAccountNotFoundForSpendReservation,
        BankAccountNotFoundForInternalTransfer,
        BankAccountEitherNotSudoOrCallerIsNotDesignatedSudo,
        SpendReservationNotFound,
        MustBeWeightedShareGroupToCalculatePortionLiquidShareCapital,
        NotEnoughFundsInReservedToAllowSpend,
        NotEnoughFundsInFreeToAllowSpend,
        NotAuthorizedToMakeWithdrawal,
        CallerIsntInControllingMembershipForWithdrawal,
        AllSpendsFromReserveMustReferenceInternalTransferNotFound,
        NoBalanceLeftToWithdrawFromThisTransferForThisAccount,
        CallerMustSatisfyBankOwnerPermissionsForSpendReservation,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {

        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        DepositIdentityNonce get(fn deposit_identity_nonce): map
            hasher(opaque_blake2_256) OnChainTreasuryID => u32;

        ReservationIdentityNonce get(fn reservation_identity_nonce): map
            hasher(opaque_blake2_256) OnChainTreasuryID => u32;

        InternalTransfersIdentityNone get(fn internal_transfers_identity_nonce): map
            hasher(opaque_blake2_256) OnChainTreasuryID => u32;

        /// Source of truth for OnChainTreasuryId uniqueness checks
        /// WARNING: do not append a prefix because the keyspace is used directly for checking uniqueness
        /// TODO: pre-reserve any known ModuleId's that could be accidentally generated that already exist elsewhere
        BankStores get(fn bank_stores): map
            hasher(opaque_blake2_256) OnChainTreasuryID => Option<BankState<WithdrawalPermissions<T::AccountId>, BalanceOf<T>>>;

        /// All deposits made into the joint bank account represented by OnChainTreasuryID
        /// - I want to use DepositInfo as a key so that I can add Option<WithdrawalPermissions<T::AccountId>> as a value when deposits eventually have deposit-specific withdrawal permissions (like for grant milestones)
        Deposits get(fn deposits): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) u32 => Option<DepositInfo<T::AccountId, IpfsReference, BalanceOf<T>>>;

        /// Spend reservations which designated a committee for formally transferring ownership to specific destination addresses
        SpendReservations get(fn spend_reservations): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) u32 => Option<ReservationInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>>;

        /// Internal transfers of control over capital that allow transfer liquidity rights to the current controller
        InternalTransfers get(fn internal_transfers): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            // TransferId
            hasher(blake2_128_concat) u32 => Option<InternalTransferInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>>;

        /// Counts up and tracks how much capital is reserved by every account in this `TimePeriod`
        /// - it might be put back to zero every time_period with relative spending data logged in an archive node offchain, used to inform future spending constraints for each member
        ReservationTracker get(fn reservation_tracker): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;

        /// TODO: combine this with the other tracker maps by adding a key prefix that is an ENUM with variants reflecting these STATEs
        InternalTransferTracker get(fn internal_transfer_tracker): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;

        /// These types of spends are for liquidation events only (ie burning shares to liquidate capital)
        /// - it tracks the total amount that has been withdrawn by this member by burning membership for example
        FreeSpendTracker get(fn free_spend_tracker): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;

        /// Tracks all (happy path) withdrawals to ensure that there is a limit on withdrawals enforced for every account in every group, even if that limit is the group's total amt
        Withdrawals get(fn withdrawals): double_map
            // (bank_id, transfer_id)
            hasher(blake2_128_concat) (OnChainTreasuryID, u32),
            // to_account
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_on_chain_bank_account_with_sudo_permissions_for_organization(
            origin,
            registered_org: u32,
            seed: BalanceOf<T>,
            sudo_acc: T::AccountId, // sole controller for the bank account
        ) -> DispatchResult {
            let seeder = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&seeder)
                || Self::is_organization_supervisor(1u32, &seeder);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterBankAccount);
            // TODO: should add check that `registered_org` is a registered organization in the `Organization` module

            let new_bank_id = Self::register_on_chain_bank_account(registered_org, seeder, seed, WithdrawalPermissions::Sudo(sudo_acc.clone()))?;
            Self::deposit_event(RawEvent::NewOnChainTreasuryRegisteredWithSudoPermissions(new_bank_id, sudo_acc));
            Ok(())
        }
        #[weight = 0]
        fn register_on_chain_bank_account_with_flat_share_group_permissions(
            origin,
            seed: BalanceOf<T>,
            organization: OrgId,
            share_id: u32,
        ) -> DispatchResult {
            let seeder = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&seeder)
                || Self::is_organization_supervisor(1u32, &seeder);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterBankAccount);

            let wrapped_share_id = ShareID::Flat(share_id);
            let new_bank_id = Self::register_on_chain_bank_account(organization, seeder, seed, WithdrawalPermissions::AnyMemberOfOrgShareGroup(organization, wrapped_share_id))?;
            Self::deposit_event(RawEvent::NewOnChainTreasuryRegisteredWithFlatShareGroupPermissions(new_bank_id, organization, wrapped_share_id));
            Ok(())
        }
        #[weight = 0]
        fn register_on_chain_bank_account_with_weighted_share_group_permissions(
            origin,
            seed: BalanceOf<T>,
            organization: OrgId,
            share_id: u32,
        ) -> DispatchResult {
            let seeder = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&seeder)
                || Self::is_organization_supervisor(1u32, &seeder);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterBankAccount);

            let wrapped_share_id = ShareID::WeightedAtomic(share_id);
            let new_bank_id = Self::register_on_chain_bank_account(organization, seeder, seed, WithdrawalPermissions::AnyMemberOfOrgShareGroup(organization, wrapped_share_id))?;
            Self::deposit_event(RawEvent::NewOnChainTreasuryRegisteredWithWeightedShareGroupPermissions(new_bank_id, organization, wrapped_share_id));
            Ok(())
        }
        #[weight = 0]
        fn deposit_from_signer_into_on_chain_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            amount: BalanceOf<T>,
            savings_tax: Option<Permill>,
            reason: IpfsReference,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;

            Self::deposit_into_bank(depositer.clone(), bank_id, amount, reason.clone())?;
            Self::deposit_event(RawEvent::CapitalDepositedIntoOnChainBankAccount(depositer, bank_id, amount, reason));
            Ok(())
        }
        #[weight = 0]
        fn sudo_withdrawal_from_on_chain_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            to: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let sudo_withdrawer = ensure_signed(origin)?;

            // succeeds if user is the authorized sudo withdrawer for this bank account
            Self::spend_from_free(WithdrawalPermissions::Sudo(sudo_withdrawer), bank_id, to.clone(), amount)?;
            Self::deposit_event(RawEvent::SudoWithdrawalFromOnChainBankAccount(bank_id, to, amount));
            Ok(())
        }
        // #[weight = 0]
        // fn burn_all_shares_to_leave_weighted_membership_bank(
        //     origin,
        //     bank_id: OnChainTreasuryID,
        // ) -> DispatchResult {
        //     let leaving_member = ensure_signed(origin)?;
        //     let amount_withdrawn_by_burning_shares = Self::withdraw_capital_by_burning_shares(bank_id, leaving_member.clone(), None)?;
        //     Self::deposit_event(RawEvent::WeightedShareGroupMemberBurnedSharesToClaimProportionalWithdrawal(bank_id, leaving_member, amount_withdrawn_by_burning_shares));
        //     Ok(())
        // }
        // #[weight = 0]
        // fn withdraw_due_portion_of_deposit_from_weighted_membership_bank(
        //     origin,
        //     bank_id: OnChainTreasuryID,
        //     deposit: DepositInfo<T::AccountId, IpfsReference, BalanceOf<T>>,
        // ) -> DispatchResult {
        //     let to_claimer = ensure_signed(origin)?;
        //     let amount_withdrawn = Self::claim_portion_of_on_chain_deposit(bank_id, deposit, to_claimer.clone(), None)?;
        //     Self::deposit_event(RawEvent::WeightedShareGroupMemberClaimedPortionOfDepositToWithdraw(bank_id, to_claimer, amount_withdrawn));
        //     Ok(())
        // }
    }
}

impl<T: Trait> Module<T> {
    fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    fn is_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, T::AccountId>>::is_sudo_account(
            who,
        )
    }
    fn is_organization_supervisor(organization: u32, who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, T::AccountId>>::is_organization_supervisor(organization, who)
    }
    fn is_share_supervisor(organization: u32, share_id: ShareID, who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, T::AccountId>>::is_share_supervisor(organization, share_id.into(), who)
    }
    /// This method simply checks membership in group,
    /// note: `WithdrawalPermissions` lacks context for magnitude requirement
    fn account_satisfies_withdrawal_permissions(
        who: &T::AccountId,
        governance_config: WithdrawalPermissions<T::AccountId>,
    ) -> bool {
        match governance_config {
            WithdrawalPermissions::Sudo(acc) => &acc == who,
            WithdrawalPermissions::AnyOfTwoAccounts(acc1, acc2) => ((&acc1 == who) || (&acc2 == who)),
            WithdrawalPermissions::AnyAccountInOrg(org_id) => {
                <<T as Trait>::Organization as OrgChecks<u32, T::AccountId>>::check_membership_in_org(org_id, who)
            },
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, wrapped_share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<u32, T::AccountId>>::check_membership_in_share_group(org_id, wrapped_share_id.into(), who)
            },
        }
    }
    // TODO: check membership in (share) group and ownership in (share) group matches some INPUT requirement (second input) (_membership_and_magnitude_)
}

impl<T: Trait> IDIsAvailable<OnChainTreasuryID> for Module<T> {
    fn id_is_available(id: OnChainTreasuryID) -> bool {
        <BankStores<T>>::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, BankAssociatedIdentifiers)> for Module<T> {
    fn id_is_available(id: (OnChainTreasuryID, BankAssociatedIdentifiers)) -> bool {
        match id.1 {
            BankAssociatedIdentifiers::Deposit(proposed_deposit_id) => {
                <Deposits<T>>::get(id.0, proposed_deposit_id).is_none()
            }
            BankAssociatedIdentifiers::Reservation(proposed_reservation_id) => {
                <SpendReservations<T>>::get(id.0, proposed_reservation_id).is_none()
            }
            BankAssociatedIdentifiers::InternalTransfer(proposed_transfer_id) => {
                <InternalTransfers<T>>::get(id.0, proposed_transfer_id).is_none()
            }
        }
    }
}

impl<T: Trait> GenerateUniqueID<(OnChainTreasuryID, BankAssociatedIdentifiers)> for Module<T> {
    fn generate_unique_id(
        proposed_id: (OnChainTreasuryID, BankAssociatedIdentifiers),
    ) -> (OnChainTreasuryID, BankAssociatedIdentifiers) {
        if !Self::id_is_available(proposed_id.clone()) {
            let mut new_id = proposed_id.1.iterate();
            while !Self::id_is_available((proposed_id.0, new_id.clone())) {
                new_id = new_id.iterate();
            }
            (proposed_id.0, new_id)
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> GenerateUniqueID<OnChainTreasuryID> for Module<T> {
    fn generate_unique_id(proposed_id: OnChainTreasuryID) -> OnChainTreasuryID {
        if !Self::id_is_available(proposed_id) {
            let mut treasury_nonce_id = BankIDNonce::get().iterate();
            while !Self::id_is_available(treasury_nonce_id) {
                treasury_nonce_id = treasury_nonce_id.iterate();
            }
            treasury_nonce_id
        } else {
            proposed_id
        }
    }
}

impl<T: Trait> OnChainBank for Module<T> {
    type OrgId = u32; // TODO: here is where I should export the OrgId type from the Organization subtype
    type TreasuryId = OnChainTreasuryID;
}

impl<T: Trait> RegisterBankAccount<T::AccountId, BalanceOf<T>> for Module<T> {
    type GovernanceConfig = WithdrawalPermissions<T::AccountId>;
    fn register_on_chain_bank_account(
        registered_org: Self::OrgId,
        from: T::AccountId,
        amount: BalanceOf<T>, // TODO: ADD MINIMUM AMOUNT TO OPEN BANK
        owner_s: Self::GovernanceConfig,
    ) -> Result<Self::TreasuryId, DispatchError> {
        let proposed_id = OnChainTreasuryID::default();
        let generated_id = Self::generate_unique_id(proposed_id);
        // default all of it is put into savings but this optional param allows us to set some aside for spends
        let new_bank = BankState::new_from_deposit(registered_org, amount, owner_s);
        let to = Self::account_id(generated_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        <BankStores<T>>::insert(generated_id, new_bank);
        Ok(generated_id)
    }
}

impl<T: Trait> OwnershipProportionCalculations<T::AccountId, BalanceOf<T>, Permill> for Module<T> {
    fn calculate_proportion_ownership_for_account(
        account: T::AccountId,
        group: Self::GovernanceConfig,
    ) -> Option<Permill> {
        None
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: Self::GovernanceConfig,
    ) -> Option<BalanceOf<T>> {
        None
    }
}

impl<T: Trait> BankDepositsAndSpends<BalanceOf<T>> for Module<T> {
    type Bank = BankState<WithdrawalPermissions<T::AccountId>, BalanceOf<T>>;
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

impl<T: Trait> DepositIntoBank<T::AccountId, IpfsReference, BalanceOf<T>> for Module<T> {
    fn deposit_into_bank(
        from: T::AccountId,
        to_bank_id: Self::TreasuryId,
        amount: BalanceOf<T>,
        reason: IpfsReference,
    ) -> DispatchResult {
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
        let unique_deposit =
            Self::generate_unique_id((to_bank_id, BankAssociatedIdentifiers::Deposit(1u32)));
        let deposit_id: u32 = unique_deposit.1.into();

        // TODO2: when will we delete this, how long is this going to stay in storage?
        <Deposits<T>>::insert(to_bank_id, deposit_id, new_deposit);
        // TODO: return DepositId?
        Ok(())
    }
}

impl<T: Trait> BankReservations<T::AccountId, BalanceOf<T>, IpfsReference> for Module<T> {
    fn reserve_for_spend(
        caller: T::AccountId, // must be in owner_s: GovernanceConfig for BankState, that's the auth
        bank_id: Self::TreasuryId,
        reason: IpfsReference,
        amount: BalanceOf<T>,
        // acceptance committee for approving set aside spends below the amount
        controller: Self::GovernanceConfig, // default WithdrawalRules
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        // check that the account is authenticated to do this in the context of this bank
        ensure!(
            Self::account_satisfies_withdrawal_permissions(&caller, bank_account.owner_s()),
            Error::<T>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
        );
        // tracks all spend reservations made by all members
        let new_reserved_sum_by_caller =
            if let Some(previous_reservations) = <ReservationTracker<T>>::get(bank_id, &caller) {
                previous_reservations + amount
            } else {
                amount
            };
        <ReservationTracker<T>>::insert(bank_id, caller, new_reserved_sum_by_caller);

        // create Reservation Info object
        let new_spend_reservation = ReservationInfo::new(reason, amount, controller);

        // change bank_account such
        Ok(())
    }
    // Allocate some funds (previously set aside for spending reasons) to be withdrawable by new group
    // - this is an internal transfer to a team and it makes this capital withdrawable by them
    fn transfer_spending_power(
        caller: T::AccountId, // must be in reference's supervision_committee
        bank_id: Self::TreasuryId,
        reason: IpfsReference,
        // reference to specific reservation
        reservation_id: u32,
        // move control of funds to new outer group which can reserve or withdraw directly
        new_controller: Self::GovernanceConfig,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForInternalTransfer)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // check caller auth in the context of the spend_reservation (to transfer spending power)
        let spend_reservation_controller = spend_reservation.controller();
        // TODO: rewrite is_member_of_formed_org for this module so for `WithdrawalPermissions`
        Ok(())
    }
}

impl<T: Trait> BankSpends<T::AccountId, BalanceOf<T>> for Module<T> {
    fn spend_from_free(
        caller: Self::GovernanceConfig,
        from_bank_id: Self::TreasuryId,
        to: T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let bank_account = <BankStores<T>>::get(from_bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForWithdrawal)?;
        // authenticate caller
        ensure!(
            bank_account.is_owner_s(caller),
            Error::<T>::NotAuthorizedToMakeWithdrawal
        );
        // update the amount stored in the bank
        let bank_after_withdrawal = Self::fallible_spend_from_free(bank_account, amount)?;
        // make the transfer
        let from = Self::account_id(from_bank_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        <BankStores<T>>::insert(from_bank_id, bank_after_withdrawal);
        Ok(amount)
    }
    fn spend_from_reserved(
        caller: Self::GovernanceConfig,
        from_bank_id: Self::TreasuryId,
        // reservation_id
        id: u32,
        to: T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        todo!()
        //Err(Error::<T>::NotAuthorizedToMakeWithdrawal.into())
    }
    // spends up to allowed amount by default
    fn spend_from_transfers(
        from_bank_id: Self::TreasuryId,
        id: u32, // refers to InternalTransfer, which transfers control over a subset of the overall funds
        to: T::AccountId,
        amount: BalanceOf<T>,
    ) -> Result<BalanceOf<T>, DispatchError> {
        let bank_account = <BankStores<T>>::get(from_bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForWithdrawal)?;
        let transfer_certificate = <InternalTransfers<T>>::get(from_bank_id, id)
            .ok_or(Error::<T>::AllSpendsFromReserveMustReferenceInternalTransferNotFound)?;
        let controller_from_certificate = transfer_certificate.controller();
        // TODO: replace this with a trait on the `TransferInfo` associated type?
        let due_amount = Self::calculate_proportional_amount_for_account(
            transfer_certificate.amount(),
            to.clone(),
            controller_from_certificate,
        )
        .ok_or(Error::<T>::CallerIsntInControllingMembershipForWithdrawal)?;
        let mut withdrawal_data = ((from_bank_id, id), to.clone(), due_amount);
        ensure!(
            due_amount >= amount,
            Error::<T>::NotEnoughFundsInReservedToAllowSpend
        );
        // check if withdrawal has occured before
        if let Some(amount_left) =
            <Withdrawals<T>>::get(withdrawal_data.0, withdrawal_data.1.clone())
        {
            ensure!(
                amount_left >= amount,
                Error::<T>::NotEnoughFundsInReservedToAllowSpend
            );
        }
        withdrawal_data.2 = due_amount - amount;
        // update the amount stored in the bank
        let bank_after_withdrawal = Self::fallible_spend_from_reserved(bank_account, amount)?;
        // make the transfer
        let from = Self::account_id(from_bank_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        <Withdrawals<T>>::insert(withdrawal_data.0, withdrawal_data.1, withdrawal_data.2);
        <BankStores<T>>::insert(from_bank_id, bank_after_withdrawal);
        Ok(amount)
    }
}

impl<T: Trait> DepositInformation<T::AccountId, BalanceOf<T>> for Module<T> {
    type DepositInfo = DepositInfo<T::AccountId, IpfsReference, BalanceOf<T>>;

    fn get_deposits_by_account(
        bank_id: Self::TreasuryId,
        depositer: T::AccountId,
    ) -> Option<Vec<Self::DepositInfo>> {
        let depositers_deposits = <Deposits<T>>::iter()
            .filter(|(id, _, deposit)| id == &bank_id && deposit.depositer() == depositer)
            .map(|(_, _, deposit)| deposit)
            .collect::<Vec<Self::DepositInfo>>();
        if depositers_deposits.is_empty() {
            None
        } else {
            Some(depositers_deposits)
        }
    }
    fn total_capital_deposited_by_account(
        bank_id: Self::TreasuryId,
        depositer: T::AccountId,
    ) -> BalanceOf<T> {
        <Deposits<T>>::iter()
            .filter(|(id, _, deposit)| id == &bank_id && deposit.depositer() == depositer)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, deposit)| {
                acc + deposit.amount()
            })
    }
}

impl<T: Trait> TransferInformation<T::AccountId, BalanceOf<T>> for Module<T> {
    type TransferInfo =
        InternalTransferInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>;
    fn get_transfers_by_account_that_invoked(
        bank_id: Self::TreasuryId,
        invoker: T::AccountId,
    ) -> Option<Vec<Self::TransferInfo>> {
        None
    }
    fn total_capital_transferred_by_account(
        bank_id: Self::TreasuryId,
        invoker: T::AccountId,
    ) -> BalanceOf<T> {
        BalanceOf::<T>::zero()
    }
}

// impl<T: Trait> OnChainWithdrawalFilters<T::AccountId, IpfsReference, BalanceOf<T>, Permill>
//     for Module<T>
// {
//     fn calculate_liquid_portion_of_on_chain_deposit(
//         from_bank_id: Self::TreasuryId,
//         deposit: Self::DepositInfo,
//         to_claimer: T::AccountId,
//     ) -> Result<BalanceOf<T>, DispatchError> {
//         // this is implied to be the withdrawable portion
//         // TODO: add the choice to withdraw capital or accept greater share ownership?
//         // - idk, I dont want to add too much confusion already, pick a default like it's set the same
//         // for everyone and they are mandated to withdraw and its reserved for them to withdraw...
//         let amount = if let Some(savings_pct) = deposit.savings_pct() {
//             let reserved_for_savings = savings_pct * deposit.amount();
//             deposit.amount() - reserved_for_savings
//         } else {
//             deposit.amount()
//         };
//         let deposit_dne = Self::id_is_available((from_bank_id, deposit));
//         ensure!(
//             !deposit_dne,
//             Error::<T>::DepositCannotBeFoundToCalculateCorrectPortion
//         );
//         // get the bank's controller information
//         let controller = <BankStores<T>>::get(from_bank_id)
//             .ok_or(Error::<T>::CannotCalculateDepositPortionFromBankThatDNE)?;
//         let org_share_id = controller
//             .extract_weighted_share_group_id()
//             .ok_or(Error::<T>::MustBeWeightedShareGroupToCalculatePortionOfOnChainDeposit)?;
//         // + 1 constant time map lookup
//         let total_shares_issued_for_group = <<T as Trait>::Organization as WeightedShareWrapper<
//             u32,
//             u32,
//             T::AccountId,
//         >>::get_outstanding_weighted_shares(
//             org_share_id.0, org_share_id.1
//         )?;
//         // + 1 constant time map lookup
//         let shares_owned_by_member = <<T as Trait>::Organization as WeightedShareWrapper<
//             u32,
//             u32,
//             T::AccountId,
//         >>::get_weighted_shares_for_member(
//             org_share_id.0, org_share_id.1, &to_claimer
//         )?;
//         let ownership_portion = Permill::from_rational_approximation(
//             shares_owned_by_member,
//             total_shares_issued_for_group,
//         );
//         // calculate the amount to withdraw;
//         let amount_to_withdraw = ownership_portion * amount;
//         Ok(amount_to_withdraw)
//     }
//     // no guarantees on the value this returns, on chain conditions change fast
//     fn calculate_liquid_share_capital_from_savings(
//         from_bank_id: Self::TreasuryId,
//         to_claimer: T::AccountId,
//     ) -> Result<(u32, u32, BalanceOf<T>), DispatchError> {
//         let bank_account = <BankStores<T>>::get(from_bank_id)
//             .ok_or(Error::<T>::CannotCalculateLiquidShareCapitalFromBankThatDNE)?;
//         // Burning Shares Only Yields Access To The Portion of SAVINGS -- it does not expose capital reserved for spends
//         // i.e. reserved for others as part of a grant milestone payment
//         let balance_withdrawable_by_burning_shares = bank_account.savings();
//         let org_share_id = bank_account
//             .extract_weighted_share_group_id()
//             .ok_or(Error::<T>::MustBeWeightedShareGroupToCalculatePortionLiquidShareCapital)?;
//         // + 1 constant time map lookup
//         let total_shares_issued_for_group = <<T as Trait>::Organization as WeightedShareWrapper<
//             u32,
//             u32,
//             T::AccountId,
//         >>::get_outstanding_weighted_shares(
//             org_share_id.0, org_share_id.1
//         )?;
//         // + 1 constant time map lookup
//         let shares_owned_by_member = <<T as Trait>::Organization as WeightedShareWrapper<
//             u32,
//             u32,
//             T::AccountId,
//         >>::get_weighted_shares_for_member(
//             org_share_id.0, org_share_id.1, &to_claimer
//         )?;
//         let ownership_portion = Permill::from_rational_approximation(
//             shares_owned_by_member,
//             total_shares_issued_for_group,
//         );
//         // note that this is only a proportion of savings, not deposits...
//         let amount_can_withdraw = ownership_portion * balance_withdrawable_by_burning_shares;
//         Ok((org_share_id.0, org_share_id.1, amount_can_withdraw))
//     }
//     // request for a portion of an on-chain deposit, the impl defines what determines the fair portion
//     fn claim_portion_of_on_chain_deposit(
//         from_bank_id: Self::TreasuryId,
//         deposit: Self::DepositInfo,
//         to_claimer: T::AccountId,
//         amount: Option<BalanceOf<T>>,
//     ) -> Result<BalanceOf<T>, DispatchError> {
//         let bank_account_dne = Self::id_is_available(from_bank_id);
//         ensure!(
//             bank_account_dne,
//             Error::<T>::CannotClaimDepositFromBankThatDNE
//         );
//         // check that they can claim some portion
//         let can_claim: BalanceOf<T> = Self::calculate_liquid_portion_of_on_chain_deposit(
//             from_bank_id,
//             deposit,
//             to_claimer.clone(),
//         )?;
//         // set the amount for withdrawal, make sure it is less than above
//         let amount_for_withdrawal = if let Some(amt) = amount {
//             ensure!(
//                 amt <= can_claim,
//                 Error::<T>::CanOnlyClaimUpToOwnershipPortionByDefault
//             );
//             amt
//         } else {
//             can_claim
//         };
//         // make withdrawal
//         let from = Self::account_id(from_bank_id);
//         T::Currency::transfer(
//             &from,
//             &to_claimer,
//             amount_for_withdrawal,
//             ExistenceRequirement::KeepAlive,
//         )?;
//         Ok(amount_for_withdrawal)
//     }
//     // irreversible decision to sell ownership in exchange for a portion of the collateral
//     // - automatically calculated according to the proportion of ownership at the time the request is processed
//     // -- NOTE: this does not shield against dilution if there is a run on the collateral because it does not yield a limit order for the share sale
//     fn withdraw_capital_by_burning_shares(
//         from_bank_id: Self::TreasuryId,
//         to_claimer: T::AccountId,
//         amount: Option<BalanceOf<T>>, // if None, burns all shares for to_claimer to liquidate as much as possible
//     ) -> Result<BalanceOf<T>, DispatchError> {
//         let bank_account_dne = Self::id_is_available(from_bank_id);
//         ensure!(
//             bank_account_dne,
//             Error::<T>::CannotClaimDepositFromBankThatDNE
//         );
//         let org_share_id_shares =
//             Self::calculate_liquid_share_capital_from_savings(from_bank_id, to_claimer.clone())?;
//         let can_withdraw = org_share_id_shares.2;
//         // if None, it burns all shares by default see last outside method call at bottom of method body
//         let mut proportion_of_own_shares_to_burn: Option<Permill> = None;
//         let amount_withdrawn = if let Some(amt) = amount {
//             ensure!(
//                 amt <= can_withdraw,
//                 Error::<T>::CannotBurnEnoughSharesToLiquidateCapitalForWithdrawalRequest
//             );
//             let proportion_of_capital_requested =
//                 Permill::from_rational_approximation(amt, can_withdraw);
//             proportion_of_own_shares_to_burn = Some(proportion_of_capital_requested);
//             amt
//         } else {
//             can_withdraw
//         };
//         // make withdrawal
//         let from = Self::account_id(from_bank_id);
//         T::Currency::transfer(
//             &from,
//             &to_claimer,
//             amount_withdrawn,
//             ExistenceRequirement::KeepAlive,
//         )?;
//         // burn proportional amount of shares
//         <<T as Trait>::Organization as WeightedShareIssuanceWrapper<
//             u32,
//             u32,
//             T::AccountId,
//             Permill,
//         >>::burn_weighted_shares_for_member(
//             org_share_id_shares.0,
//             org_share_id_shares.1,
//             to_claimer,
//             proportion_of_own_shares_to_burn,
//         )?;
//         Ok(amount_withdrawn)
//     }
// }
