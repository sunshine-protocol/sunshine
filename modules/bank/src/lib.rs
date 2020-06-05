#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! This module defines rules for withdrawal from a joint account; enables
//! registered organizations to manage these accounts with share permission groups.

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use util::{
    bank::{
        BankMapID, BankState, BankTrackerID, BankTrackerIdentifier, CommitmentInfo, DepositInfo,
        InternalTransferInfo, OnChainTreasuryID, ReservationInfo, WithdrawalPermissions,
    },
    organization::ShareID,
    traits::{
        AccessProfile, BankDepositsAndSpends, BankReservations, BankSpends, BankStorageInfo,
        CheckBankBalances, CommitAndTransfer, CommitSpendReservation, DepositIntoBank,
        DepositSpendOps, FreeToReserved, GenerateUniqueID, GetInnerOuterShareGroups, IDIsAvailable,
        MoveFundsOutCommittedOnly, MoveFundsOutUnCommittedOnly, OnChainBank, OrgChecks,
        OrganizationDNS, OwnershipProportionCalculations, RegisterBankAccount, RegisterShareGroup,
        SeededGenerateUniqueID, ShareGroupChecks, SupervisorPermissions, TermSheetExit,
        WeightedShareIssuanceWrapper, WeightedShareWrapper,
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

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>;

    type Organization: OrgChecks<u32, Self::AccountId>
        + ShareGroupChecks<u32, ShareID, Self::AccountId>
        + GetInnerOuterShareGroups<u32, ShareID, Self::AccountId>
        + SupervisorPermissions<u32, ShareID, Self::AccountId>
        + WeightedShareWrapper<u32, u32, Self::AccountId>
        + WeightedShareIssuanceWrapper<u32, u32, Self::AccountId, Permill>
        + RegisterShareGroup<u32, ShareID, Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<u32, Self::AccountId, IpfsReference>;

    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        // u32 is the hosting organization identifier
        RegisteredNewOnChainBank(AccountId, OnChainTreasuryID, Balance, u32, WithdrawalPermissions<AccountId>),
        CapitalDepositedIntoOnChainBankAccount(AccountId, OnChainTreasuryID, Balance, IpfsReference),
        // u32 is reservation_id
        SpendReservedForBankAccount(AccountId, OnChainTreasuryID, u32, IpfsReference, Balance, WithdrawalPermissions<AccountId>),
        CommitSpendBeforeInternalTransfer(AccountId, OnChainTreasuryID, u32, IpfsReference, Balance, WithdrawalPermissions<AccountId>),
        UnReserveUncommittedReservationToMakeFree(AccountId, OnChainTreasuryID, u32, Balance),
        UnReserveCommittedReservationToMakeFree(AccountId, OnChainTreasuryID, u32, Balance),
        InternalTransferExecutedAndSpendingPowerDoledOutToController(AccountId, OnChainTreasuryID, IpfsReference, u32, Balance, WithdrawalPermissions<AccountId>),
        SpendRequestForInternalTransferApprovedAndExecuted(OnChainTreasuryID, AccountId, Balance, u32),
        AccountLeftMembershipAndWithdrewProportionOfFreeCapitalInBank(OnChainTreasuryID, AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        BankAccountNotFoundForTermSheetExit,
        MustHaveCertainAuthorityToRegisterBankAccount,
        MustBeWeightedShareGroupToCalculatePortionOfOnChainDeposit,
        MustHaveOwnershipToRageQuit,
        BankAccountNotFoundForDeposit,
        BankAccountNotFoundForWithdrawal,
        BankAccountNotFoundForSpendReservation,
        BankAccountNotFoundForInternalTransfer,
        SpendReservationNotFound,
        NotEnoughFundsInReservedToAllowSpend,
        CannotCommitReservedSpendBecauseAmountExceedsSpendReservation,
        NotEnoughFundsInFreeToAllowSpend,
        NotEnoughFundsInFreeToAllowReservation,
        CallerIsntInControllingMembershipForWithdrawal,
        AllSpendsFromReserveMustReferenceInternalTransferNotFound,
        CallerMustSatisfyBankOwnerPermissionsForSpendReservation,
        NotEnoughFundsCommittedToSatisfyUnreserveAndFreeRequest,
        NotEnoughFundsCommittedToEnableInternalTransfer,
        NotEnoughFundsInSpendReservationUnCommittedToSatisfyUnreserveUnCommittedRequest,
        NotEnoughFundsInBankReservedToSatisfyUnReserveUnComittedRequest,
        InternalTransferRequestExceedsReferencedSpendCommitment,
        SpendCommitmentNotFoundForInternalTransfer,
        GovernanceConfigDoesNotSatisfyOrgRequirementsForBankRegistration,
        RegistrationMustDepositAboveModuleMinimum,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {

        BankIDNonce get(fn bank_id_nonce): OnChainTreasuryID;

        BankAssociatedNonces get(fn bank_associated_nonces): double_map
            hasher(opaque_blake2_256) OnChainTreasuryID,
            hasher(opaque_blake2_256) BankMapID => u32;

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

        /// Spend commitments, marks formal shift of power over capital from direct bank.controller() to reservation.controller()
        SpendCommitments get(fn spend_commitments): double_map
            hasher(blake2_128_concat) BankTrackerIdentifier, // only the CommitSpend variant allowed, seems unergonomic but it works
            hasher(blake2_128_concat) WithdrawalPermissions<T::AccountId> => Option<CommitmentInfo<IpfsReference, BalanceOf<T>>>;

        /// Internal transfers of control over capital that allow transfer liquidity rights to the current controller
        InternalTransfers get(fn internal_transfers): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            // TransferId
            hasher(blake2_128_concat) u32 => Option<InternalTransferInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>>;

        /// This storage item is important for tracking how many transfers each AccountId has made and possibly placing restrictions on that based on activity norms
        BankTracker get(fn bank_tracker): double_map
            hasher(blake2_128_concat) BankTrackerIdentifier,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        /// Infallible deposit runtime method invokes a helper trait
        /// method exposed outside this runtime method to make this
        /// method accessible from inherited modules.
        #[weight = 0]
        fn deposit_from_signer_for_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            amount: BalanceOf<T>,
            //savings_tax: Option<Permill>, // to be added ;)
            reason: IpfsReference,
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
            hosting_org: OrgId, // pre-requisite is registered organization
            bank_controller: WithdrawalPermissions<T::AccountId>,
        ) -> DispatchResult {
            let seeder = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&seeder)
                || Self::is_organization_supervisor(hosting_org, &seeder);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterBankAccount);

            let new_bank_id = Self::register_on_chain_bank_account(hosting_org, seeder.clone(), seed, bank_controller.clone())?;
            Self::deposit_event(RawEvent::RegisteredNewOnChainBank(seeder, new_bank_id, seed, hosting_org, bank_controller));
            Ok(())
        }
        #[weight = 0]
        fn reserve_spend_for_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            reason: IpfsReference,
            amount: BalanceOf<T>,
            controller: WithdrawalPermissions<T::AccountId>,
        ) -> DispatchResult {
            let reserver = ensure_signed(origin)?;
            // auth happens in this method because the context for authentication requires the bank_account object in storage
            let new_reservation_id = Self::reserve_for_spend(reserver.clone(), bank_id, reason.clone(), amount, controller.clone())?;
            Self::deposit_event(RawEvent::SpendReservedForBankAccount(reserver, bank_id, new_reservation_id, reason, amount, controller));
            Ok(())
        }
        #[weight = 0]
        fn commit_reserved_spend_for_transfer_inside_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: u32,
            reason: IpfsReference,
            amount: BalanceOf<T>,
            future_controller: WithdrawalPermissions<T::AccountId>,
        ) -> DispatchResult {
            let committer = ensure_signed(origin)?;
            // auth happens in this method because the context for authentication requires the bank_account object in storage
            Self::commit_reserved_spend_for_transfer(committer.clone(), bank_id, reservation_id, reason.clone(), amount, future_controller.clone())?;
            Self::deposit_event(RawEvent::CommitSpendBeforeInternalTransfer(committer, bank_id, reservation_id, reason, amount, future_controller));
            Ok(())
        }
        #[weight = 0]
        fn unreserve_uncommitted_reservation_to_make_free(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: u32,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let qualified_bank_controller = ensure_signed(origin)?;
            // auth happens in this method because the context for authentication requires the bank_account object in storage
            Self::unreserve_uncommitted_to_make_free(qualified_bank_controller.clone(), bank_id, reservation_id, amount)?;
            Self::deposit_event(RawEvent::UnReserveUncommittedReservationToMakeFree(qualified_bank_controller, bank_id, reservation_id, amount));
            Ok(())
        }
        #[weight = 0]
        fn unreserve_committed_reservation_to_make_free(
            origin,
            bank_id: OnChainTreasuryID,
            reservation_id: u32,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let qualified_spend_reservation_controller = ensure_signed(origin)?;
            // auth happens in this method because the context for authentication requires the bank_account object in storage
            Self::unreserve_committed_to_make_free(qualified_spend_reservation_controller.clone(), bank_id, reservation_id, amount)?;
            Self::deposit_event(RawEvent::UnReserveCommittedReservationToMakeFree(qualified_spend_reservation_controller, bank_id, reservation_id, amount));
            Ok(())
        }
        #[weight = 0]
        fn transfer_spending_for_spend_commitment(
            origin,
            bank_id: OnChainTreasuryID,
            reason: IpfsReference,
            reservation_id: u32,
            amount: BalanceOf<T>,
            committed_controller: WithdrawalPermissions<T::AccountId>,
        ) -> DispatchResult {
            let qualified_spend_reservation_controller = ensure_signed(origin)?;
            // auth happens in this method because the context for authentication requires the bank_account object in storage
            Self::transfer_spending_power(qualified_spend_reservation_controller.clone(), bank_id, reason.clone(), reservation_id, amount, committed_controller.clone())?;
            Self::deposit_event(RawEvent::InternalTransferExecutedAndSpendingPowerDoledOutToController(qualified_spend_reservation_controller, bank_id, reason, reservation_id, amount, committed_controller));
            Ok(())
        }
        #[weight = 0]
        fn withdraw_by_referencing_internal_transfer(
            origin,
            bank_id: OnChainTreasuryID,
            transfer_id: u32,
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
    fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    fn is_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, ShareID, T::AccountId>>::is_sudo_account(
            who,
        )
    }
    fn is_organization_supervisor(organization: u32, who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, ShareID, T::AccountId>>::is_organization_supervisor(organization, who)
    }
    // fn is_share_supervisor(organization: u32, share_id: ShareID, who: &T::AccountId) -> bool {
    //     <<T as Trait>::Organization as SupervisorPermissions<u32, T::AccountId>>::is_share_supervisor(organization, share_id.into(), who)
    // }
    /// This helper just checks existence in context of inherited modules
    /// -> NO checks are done on the AccountIds because no context is provided
    fn governance_config_satisfies_org_controller_registration_requirements(
        org: u32,
        governance_config: WithdrawalPermissions<T::AccountId>,
    ) -> bool {
        // check the existence of the governance_config in the context of the org
        match governance_config {
            WithdrawalPermissions::AnyOfTwoAccounts(_, _) => true, // no restrictions for now
            WithdrawalPermissions::AnyAccountInOrg(org_id) => {
                org_id == org && // can only create banks with controllers for this org
                <<T as Trait>::Organization as OrgChecks<u32, T::AccountId>>::check_org_existence(org)
            },
            // HAPPY PATH, pls use this for bank controllers for structured liquidity from account.free() as well
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, wrapped_share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<u32, ShareID, T::AccountId>>::check_share_group_existence(org_id, wrapped_share_id)
            },
        }
        // a more granular check might require inner share membership for example in the org
    }
    /// This method simply checks membership in group,
    /// note: `WithdrawalPermissions` lacks context for magnitude requirement
    fn account_satisfies_withdrawal_permissions(
        who: &T::AccountId,
        governance_config: WithdrawalPermissions<T::AccountId>,
    ) -> bool {
        match governance_config {
            WithdrawalPermissions::AnyOfTwoAccounts(acc1, acc2) => ((&acc1 == who) || (&acc2 == who)),
            WithdrawalPermissions::AnyAccountInOrg(org_id) => {
                <<T as Trait>::Organization as OrgChecks<u32, T::AccountId>>::check_membership_in_org(org_id, who)
            },
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, wrapped_share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<u32, ShareID, T::AccountId>>::check_membership_in_share_group(org_id, wrapped_share_id, who)
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

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, BankMapID, u32)> for Module<T> {
    fn id_is_available(id: (OnChainTreasuryID, BankMapID, u32)) -> bool {
        match id.1 {
            BankMapID::Deposit => <Deposits<T>>::get(id.0, id.2).is_none(),
            BankMapID::Reservation => <SpendReservations<T>>::get(id.0, id.2).is_none(),
            BankMapID::InternalTransfer => <InternalTransfers<T>>::get(id.0, id.2).is_none(),
        }
    }
}

impl<T: Trait> SeededGenerateUniqueID<u32, (OnChainTreasuryID, BankMapID)> for Module<T> {
    fn seeded_generate_unique_id(seed: (OnChainTreasuryID, BankMapID)) -> u32 {
        let mut new_id = <BankAssociatedNonces>::get(seed.0, seed.1) + 1u32;
        while !Self::id_is_available((seed.0, seed.1, new_id)) {
            new_id += 1u32;
        }
        <BankAssociatedNonces>::insert(seed.0, seed.1, new_id);
        new_id
    }
}

impl<T: Trait> GenerateUniqueID<OnChainTreasuryID> for Module<T> {
    fn generate_unique_id() -> OnChainTreasuryID {
        let mut treasury_nonce_id = BankIDNonce::get().iterate();
        while !Self::id_is_available(treasury_nonce_id) {
            treasury_nonce_id = treasury_nonce_id.iterate();
        }
        BankIDNonce::put(treasury_nonce_id);
        treasury_nonce_id
    }
}

impl<T: Trait> OnChainBank for Module<T> {
    type OrgId = u32; // TODO: here is where I should export the OrgId type from the Organization subtype once it is added as an associated type
    type TreasuryId = OnChainTreasuryID;
}

impl<T: Trait> RegisterBankAccount<T::AccountId, WithdrawalPermissions<T::AccountId>, BalanceOf<T>>
    for Module<T>
{
    fn register_on_chain_bank_account(
        registered_org: Self::OrgId,
        from: T::AccountId,
        amount: BalanceOf<T>, // TODO: ADD MINIMUM AMOUNT TO OPEN BANK
        owner_s: WithdrawalPermissions<T::AccountId>,
    ) -> Result<Self::TreasuryId, DispatchError> {
        ensure!(
            amount >= T::MinimumInitialDeposit::get(),
            Error::<T>::RegistrationMustDepositAboveModuleMinimum
        );
        // check that the owner_s is in the `registered_org` because those are our tacit requirements
        // => cannot create bank and assign an outside organization or outside share group its controller permissions
        ensure!(
            Self::governance_config_satisfies_org_controller_registration_requirements(
                registered_org,
                owner_s.clone()
            ),
            Error::<T>::GovernanceConfigDoesNotSatisfyOrgRequirementsForBankRegistration
        );
        // default all of it is put into savings but this optional param allows us to set some aside for spends
        let new_bank = BankState::new_from_deposit(registered_org, amount, owner_s);
        // TODO: this changes storage even if the transfer fails, is it a bug?
        let generated_id = Self::generate_unique_id();
        let to = Self::account_id(generated_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        <BankStores<T>>::insert(generated_id, new_bank);
        Ok(generated_id)
    }
    fn check_bank_owner(bank_id: Self::TreasuryId, org: Self::OrgId) -> bool {
        if let Some(account) = <BankStores<T>>::get(bank_id) {
            account.registered_org() == org
        } else {
            false
        }
    }
}

impl<T: Trait>
    OwnershipProportionCalculations<
        T::AccountId,
        WithdrawalPermissions<T::AccountId>,
        BalanceOf<T>,
        Permill,
    > for Module<T>
{
    // TODO: this is a good example of when to transition to a Result from an Option because
    // when it returns None, we don't really provide great context on why...
    fn calculate_proportion_ownership_for_account(
        account: T::AccountId,
        group: WithdrawalPermissions<T::AccountId>,
    ) -> Option<Permill> {
        match group {
            WithdrawalPermissions::AnyOfTwoAccounts(acc1, acc2) => {
                // assumes that we never use this with acc1 == acc2; use sudo in that situation
                if acc1 == account || acc2 == account {
                    Some(Permill::from_percent(50))
                } else {
                    None
                }
            }
            WithdrawalPermissions::AnyAccountInOrg(org_id) => {
                let organization_size = <<T as Trait>::Organization as OrgChecks<
                    u32,
                    T::AccountId,
                >>::get_org_size(org_id);
                Some(Permill::from_rational_approximation(1, organization_size))
            }
            WithdrawalPermissions::AnyMemberOfOrgShareGroup(org_id, wrapped_share_id) => {
                match wrapped_share_id {
                    ShareID::Flat(share_id) => {
                        let share_group_size = <<T as Trait>::Organization as ShareGroupChecks<
                            u32,
                            ShareID,
                            T::AccountId,
                        >>::get_share_group_size(
                            org_id, ShareID::Flat(share_id)
                        );
                        Some(Permill::from_rational_approximation(1, share_group_size))
                    }
                    ShareID::Weighted(share_id) => {
                        // get total stares
                        let some_total_shares =
                            <<T as Trait>::Organization as WeightedShareWrapper<
                                u32,
                                u32,
                                T::AccountId,
                            >>::get_outstanding_weighted_shares(
                                org_id, share_id
                            );
                        if let Some(total_shares) = some_total_shares {
                            let account_share_profile =
                                <<T as Trait>::Organization as WeightedShareWrapper<
                                    u32,
                                    u32,
                                    T::AccountId,
                                >>::get_member_share_profile(
                                    org_id, share_id, &account
                                );
                            if let Some(profile) = account_share_profile {
                                Some(Permill::from_rational_approximation(
                                    profile.total(),
                                    total_shares,
                                ))
                            } else {
                                // => member share profile DNE
                                None
                            }
                        } else {
                            // => share group DNE
                            None
                        }
                    }
                }
            }
        }
    }
    fn calculate_proportional_amount_for_account(
        amount: BalanceOf<T>,
        account: T::AccountId,
        group: WithdrawalPermissions<T::AccountId>,
    ) -> Option<BalanceOf<T>> {
        if let Some(ownership_pct) =
            Self::calculate_proportion_ownership_for_account(account, group)
        {
            let proportional_amount = ownership_pct * amount;
            Some(proportional_amount)
        } else {
            None
        }
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

impl<T: Trait>
    DepositIntoBank<T::AccountId, WithdrawalPermissions<T::AccountId>, IpfsReference, BalanceOf<T>>
    for Module<T>
{
    fn deposit_into_bank(
        from: T::AccountId,
        to_bank_id: Self::TreasuryId,
        amount: BalanceOf<T>,
        reason: IpfsReference,
    ) -> Result<u32, DispatchError> {
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
        // return DepositId?
        Ok(deposit_id)
    }
}

impl<T: Trait>
    BankReservations<T::AccountId, WithdrawalPermissions<T::AccountId>, BalanceOf<T>, IpfsReference>
    for Module<T>
{
    fn reserve_for_spend(
        caller: T::AccountId, // must be in owner_s: GovernanceConfig for BankState, that's the auth
        bank_id: Self::TreasuryId,
        reason: IpfsReference,
        amount: BalanceOf<T>,
        // acceptance committee for approving set aside spends below the amount
        controller: WithdrawalPermissions<T::AccountId>, // default WithdrawalRules
    ) -> Result<u32, DispatchError> {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        // check that the account is authenticated to do this in the context of this bank
        ensure!(
            Self::account_satisfies_withdrawal_permissions(&caller, bank_account.owner_s()),
            Error::<T>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
        );
        let bank_tracker_id = BankTrackerIdentifier::new(bank_id, BankTrackerID::ReservedSpend);
        // tracks all spend reservations made by all members
        let new_reserved_sum_by_caller = if let Some(previous_reservations) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            previous_reservations + amount
        } else {
            amount
        };
        // create Reservation Info object with 100 percent of it uncommitted
        let new_spend_reservation = ReservationInfo::new(reason, amount, controller);

        // change bank_account such free is less and reserved is more
        let new_bank = bank_account
            .move_from_free_to_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsInFreeToAllowReservation)?;
        let reservation_id: u32 =
            Self::seeded_generate_unique_id((bank_id, BankMapID::Reservation));
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank);
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_reserved_sum_by_caller);
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        Ok(reservation_id)
    }
    // only reserve.controller() can unreserve funds after commitment (with method further down)
    // - this method puts the funds out of reach of bank.controller() (at least immediate reach)
    fn commit_reserved_spend_for_transfer(
        caller: T::AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        reason: IpfsReference,
        amount: BalanceOf<T>,
        expected_future_owner: WithdrawalPermissions<T::AccountId>,
    ) -> DispatchResult {
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // permissions are that the caller is in the permissions of the spend_reservation
        ensure!(
            Self::account_satisfies_withdrawal_permissions(&caller, spend_reservation.controller()),
            Error::<T>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
        );
        // only commit the reserved part
        let reservation_after_commit = spend_reservation
            .commit_spend_reservation(amount)
            .ok_or(Error::<T>::CannotCommitReservedSpendBecauseAmountExceedsSpendReservation)?; // TODO better error message here
        let bank_tracker_id =
            BankTrackerIdentifier::new(bank_id, BankTrackerID::CommitSpend(reservation_id));
        // tracks all spend commitments made by specific AccountIds
        let new_committed_sum_by_caller = if let Some(previous_spend_commitments) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            previous_spend_commitments + amount
        } else {
            amount
        };
        // tracks all spend commitments made to specific WithdrawalPermissions
        let commit_amt = if let Some(existing_commitment_amt) =
            <SpendCommitments<T>>::get(bank_tracker_id.clone(), expected_future_owner.clone())
        {
            existing_commitment_amt.amount() + amount
        } else {
            amount
        };
        let new_commitment = CommitmentInfo::new(reason, commit_amt);
        // respective insertions (3)
        <SpendCommitments<T>>::insert(
            bank_tracker_id.clone(),
            expected_future_owner,
            new_commitment,
        );
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_committed_sum_by_caller);
        <SpendReservations<T>>::insert(bank_id, reservation_id, reservation_after_commit);
        Ok(())
    }
    // bank controller can unreserve if not committed
    fn unreserve_uncommitted_to_make_free(
        caller: T::AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        amount: BalanceOf<T>,
    ) -> DispatchResult {
        let bank_account = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForSpendReservation)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // this request must be approved by unreserving from the spend_reservation's
        // uncommitted funds
        let new_spend_reservation = spend_reservation
            .move_funds_out_uncommitted_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsInSpendReservationUnCommittedToSatisfyUnreserveUnCommittedRequest)?;
        // anyone in bank.controller() can make the _unreservation_ request
        ensure!(
            Self::account_satisfies_withdrawal_permissions(&caller, bank_account.owner_s()),
            Error::<T>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation
        );
        // the change in the bank account is equivalent to spending reserved and increasing free by the same amount
        let new_bank_account = bank_account
            .spend_from_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsInBankReservedToSatisfyUnReserveUnComittedRequest)?
            .deposit_into_free(amount);
        // create bank tracker identifier
        let bank_tracker_id = BankTrackerIdentifier::new(
            bank_id,
            BankTrackerID::UnReservedSpendFromUnCommitted(reservation_id),
        );
        let new_bank_tracker_amount = if let Some(existing_balance) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            // here is where you might enforce some limit per account with an ensure check
            existing_balance + amount
        } else {
            amount
        };
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank_account);
        // insert update spend reservation object (with the new, lower amount reserved)
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        // insert new bank tracker info
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_bank_tracker_amount);
        Ok(())
    }
    // reservation.controller() can unreserve committed funds
    fn unreserve_committed_to_make_free(
        caller: T::AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
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
        // same permissions for reservation; must be in the controller set of the bank to reverse a reservation
        ensure!(
            Self::account_satisfies_withdrawal_permissions(&caller, spend_reservation.controller()),
            Error::<T>::CallerMustSatisfyBankOwnerPermissionsForSpendReservation // TODO: change this error
        );
        // the change in the bank account is equivalent to spending reserved and increasing free by the same amount
        let new_bank_account = bank_account
            .spend_from_reserved(amount)
            .ok_or(Error::<T>::NotEnoughFundsCommittedToSatisfyUnreserveAndFreeRequest)?
            .deposit_into_free(amount);
        // create bank tracker identifier
        let bank_tracker_id = BankTrackerIdentifier::new(
            bank_id,
            BankTrackerID::UnReservedSpendFromCommitted(reservation_id),
        );
        let new_bank_tracker_amount = if let Some(existing_balance) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            // here is where you might enforce some limit per account with an ensure check
            existing_balance + amount
        } else {
            amount
        };
        // insert new bank account
        <BankStores<T>>::insert(bank_id, new_bank_account);
        // insert update spend reservation object (with the new, lower amount reserved)
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        // insert new bank tracker info
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_bank_tracker_amount);
        Ok(())
    }
    // Allocate some funds (previously set aside for spending reasons) to be withdrawable by new group
    // - this is an internal transfer to a team and it makes this capital withdrawable by them
    fn transfer_spending_power(
        caller: T::AccountId,
        bank_id: Self::TreasuryId,
        reason: IpfsReference,
        // reference to specific reservation
        reservation_id: u32,
        amount: BalanceOf<T>,
        // move control of funds to new outer group which can reserve or withdraw directly
        new_controller: WithdrawalPermissions<T::AccountId>,
    ) -> Result<u32, DispatchError> {
        // no authentication but the caller is logged in the BankTracker
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForInternalTransfer)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        let commitment_tracker =
            BankTrackerIdentifier::new(bank_id, BankTrackerID::CommitSpend(reservation_id));
        let spend_commitment =
            <SpendCommitments<T>>::get(commitment_tracker.clone(), new_controller.clone())
                .ok_or(Error::<T>::SpendCommitmentNotFoundForInternalTransfer)?;
        // ensure the spend_commitment exceeds the amount
        ensure!(
            spend_commitment.amount() >= amount,
            Error::<T>::InternalTransferRequestExceedsReferencedSpendCommitment
        ); // equivalent to CheckedSub
        let new_spend_commitment_amt = spend_commitment.amount() - amount;
        let new_spend_commitment =
            CommitmentInfo::new(spend_commitment.reason(), new_spend_commitment_amt);
        // ensure that the amount is less than the spend reservation amount
        let new_spend_reservation = spend_reservation
            .move_funds_out_committed_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsCommittedToEnableInternalTransfer)?;
        let bank_tracker_id = BankTrackerIdentifier::new(
            bank_id,
            BankTrackerID::InternalTransferMade(reservation_id),
        );
        let new_bank_tracker_amount = if let Some(existing_balance) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            // here is where you might enforce some limit per account with an ensure check
            existing_balance + amount
        } else {
            amount
        };
        // form a transfer_info
        let new_transfer =
            InternalTransferInfo::new(reservation_id, reason, amount, new_controller.clone());
        // generate the unique transfer_id
        let new_transfer_id: u32 =
            Self::seeded_generate_unique_id((bank_id, BankMapID::InternalTransfer));
        // insert transfer_info, thereby unlocking the capital for the `new_controller` group
        <InternalTransfers<T>>::insert(bank_id, new_transfer_id, new_transfer);
        // insert update reservation info after the transfer was made
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        // update spend commitment data
        <SpendCommitments<T>>::insert(commitment_tracker, new_controller, new_spend_commitment);
        // insert new bank tracker info
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_bank_tracker_amount);
        Ok(new_transfer_id)
    }
}

impl<T: Trait>
    CommitAndTransfer<
        T::AccountId,
        WithdrawalPermissions<T::AccountId>,
        BalanceOf<T>,
        IpfsReference,
    > for Module<T>
{
    // in one step, needs to be permissioned to a sudo of a power outside of this module
    // -> NO PERMISSIONS CHECK IN THIS METHOD
    fn commit_and_transfer_spending_power(
        caller: T::AccountId,
        bank_id: Self::TreasuryId,
        reservation_id: u32,
        reason: IpfsReference,
        amount: BalanceOf<T>,
        new_controller: WithdrawalPermissions<T::AccountId>,
    ) -> Result<u32, DispatchError> {
        let _ = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankAccountNotFoundForInternalTransfer)?;
        let spend_reservation = <SpendReservations<T>>::get(bank_id, reservation_id)
            .ok_or(Error::<T>::SpendReservationNotFound)?;
        // ensure that the amount is less than the spend reservation amount
        let new_spend_reservation = spend_reservation
            .move_funds_out_uncommitted_only(amount) // notably does not reach into committed funds!
            .ok_or(Error::<T>::NotEnoughFundsCommittedToEnableInternalTransfer)?;
        let bank_tracker_id = BankTrackerIdentifier::new(
            bank_id,
            BankTrackerID::InternalTransferMade(reservation_id),
        );
        let new_bank_tracker_amount = if let Some(existing_balance) =
            <BankTracker<T>>::get(bank_tracker_id.clone(), &caller)
        {
            // here is where you might enforce some limit per account with an ensure check
            existing_balance + amount
        } else {
            amount
        };
        // form a transfer_info
        let new_transfer =
            InternalTransferInfo::new(reservation_id, reason, amount, new_controller);
        // generate the unique transfer_id
        let new_transfer_id: u32 =
            Self::seeded_generate_unique_id((bank_id, BankMapID::InternalTransfer));
        // insert transfer_info, thereby unlocking the capital for the `new_controller` group
        <InternalTransfers<T>>::insert(bank_id, new_transfer_id, new_transfer);
        // insert update reservation info after the transfer was made
        <SpendReservations<T>>::insert(bank_id, reservation_id, new_spend_reservation);
        // insert new bank tracker info
        <BankTracker<T>>::insert(bank_tracker_id, caller, new_bank_tracker_amount);
        Ok(new_transfer_id)
    }
}

impl<T: Trait> BankSpends<T::AccountId, WithdrawalPermissions<T::AccountId>, BalanceOf<T>>
    for Module<T>
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
        id: u32, // refers to InternalTransfer, which transfers control over a subset of the overall funds
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
        )
        .ok_or(Error::<T>::CallerIsntInControllingMembershipForWithdrawal)?;
        ensure!(
            due_amount >= amount,
            Error::<T>::NotEnoughFundsInReservedToAllowSpend
        );
        let new_transfer_certificate = transfer_certificate
            .move_funds_out_committed_only(amount)
            .ok_or(Error::<T>::NotEnoughFundsInReservedToAllowSpend)?;
        let bank_tracker_id =
            BankTrackerIdentifier::new(from_bank_id, BankTrackerID::SpentFromReserved(id));
        // check if withdrawal has occurred before
        let new_due_amount =
            if let Some(amount_left) = <BankTracker<T>>::get(bank_tracker_id.clone(), to.clone()) {
                ensure!(
                    amount_left >= amount,
                    Error::<T>::NotEnoughFundsInReservedToAllowSpend
                );
                amount_left - amount
            } else {
                due_amount - amount
            };
        // update the bank store
        let bank_after_withdrawal = Self::fallible_spend_from_reserved(bank_account, amount)?;
        // make the transfer
        let from = Self::account_id(from_bank_id);
        T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;
        // insert updated transfer certificate after amount is spent
        <InternalTransfers<T>>::insert(from_bank_id, id, new_transfer_certificate);
        <BankTracker<T>>::insert(bank_tracker_id, to, new_due_amount);
        <BankStores<T>>::insert(from_bank_id, bank_after_withdrawal);
        Ok(amount)
    }
}

impl<T: Trait> BankStorageInfo<T::AccountId, WithdrawalPermissions<T::AccountId>, BalanceOf<T>>
    for Module<T>
{
    type DepositInfo = DepositInfo<T::AccountId, IpfsReference, BalanceOf<T>>;
    type ReservationInfo =
        ReservationInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>;
    type TransferInfo =
        InternalTransferInfo<IpfsReference, BalanceOf<T>, WithdrawalPermissions<T::AccountId>>;
    // deposit
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
    // reservation
    fn get_amount_left_in_spend_reservation(
        bank_id: Self::TreasuryId,
        reservation_id: u32,
    ) -> Option<BalanceOf<T>> {
        if let Some(spend_reservation) = <SpendReservations<T>>::get(bank_id, reservation_id) {
            Some(spend_reservation.amount())
        } else {
            None
        }
    }
    fn get_reservations_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: WithdrawalPermissions<T::AccountId>,
    ) -> Option<Vec<Self::ReservationInfo>> {
        let ret = <SpendReservations<T>>::iter()
            .filter(|(id, _, reservation)| id == &bank_id && reservation.controller() == invoker)
            .map(|(_, _, reservation)| reservation)
            .collect::<Vec<Self::ReservationInfo>>();
        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }
    fn total_capital_reserved_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: WithdrawalPermissions<T::AccountId>,
    ) -> BalanceOf<T> {
        <SpendReservations<T>>::iter()
            .filter(|(id, _, reservation)| id == &bank_id && reservation.controller() == invoker)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, reservation)| {
                acc + reservation.amount()
            })
    }
    // transfers
    fn get_amount_left_in_approved_transfer(
        bank_id: Self::TreasuryId,
        transfer_id: u32,
    ) -> Option<BalanceOf<T>> {
        if let Some(internal_transfer) = <InternalTransfers<T>>::get(bank_id, transfer_id) {
            Some(internal_transfer.amount())
        } else {
            None
        }
    }
    fn get_transfers_for_governance_config(
        bank_id: Self::TreasuryId,
        invoker: WithdrawalPermissions<T::AccountId>,
    ) -> Option<Vec<Self::TransferInfo>> {
        let ret = <InternalTransfers<T>>::iter()
            .filter(|(id, _, transfer)| id == &bank_id && transfer.controller() == invoker)
            .map(|(_, _, transfer)| transfer)
            .collect::<Vec<Self::TransferInfo>>();
        if ret.is_empty() {
            None
        } else {
            Some(ret)
        }
    }
    fn total_capital_transferred_to_governance_config(
        bank_id: Self::TreasuryId,
        invoker: WithdrawalPermissions<T::AccountId>,
    ) -> BalanceOf<T> {
        <InternalTransfers<T>>::iter()
            .filter(|(id, _, transfer)| id == &bank_id && transfer.controller() == invoker)
            .fold(BalanceOf::<T>::zero(), |acc, (_, _, transfer)| {
                acc + transfer.amount()
            })
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
        let org_share_id = bank_account
            .owner_s()
            .extract_org_weighted_share_id()
            .ok_or(Error::<T>::MustBeWeightedShareGroupToCalculatePortionOfOnChainDeposit)?;
        let withdrawal_amount = Self::calculate_proportional_amount_for_account(
            bank_account.free(),
            rage_quitter.clone(),
            bank_account.owner_s(),
        )
        .ok_or(Error::<T>::MustHaveOwnershipToRageQuit)?;
        // here is where we might apply some discount based on a vesting period/schedule

        // TODO: this call duplicates the Get for BankStore and we only should use one GET
        Self::spend_from_free(bank_id, rage_quitter.clone(), withdrawal_amount)?;
        // burn the shares
        let _ = <<T as Trait>::Organization as WeightedShareIssuanceWrapper<
            u32,
            u32,
            T::AccountId,
            Permill,
        >>::burn_weighted_shares_for_member(
            org_share_id.0, org_share_id.1, rage_quitter, None
        )?;
        Ok(withdrawal_amount)
    }
}
