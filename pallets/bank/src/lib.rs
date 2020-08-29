#![recursion_limit = "256"]
//! # Bank Module
//! This module expresses a joint bank account with democratic escrow rules
//! via governance by org vote
//!
//! - [`bank::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet allows orgs to govern a pool of capital.
//!
//! [`Call`]: ./enum.Call.html
//! [`Trait`]: ./trait.Trait.html
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    storage::{
        IterableStorageDoubleMap,
        IterableStorageMap,
    },
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        ReservableCurrency,
    },
    Parameter,
};
use frame_system::ensure_signed;
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
    ModuleId,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    bank::{
        BankState,
        SpendProposal,
        SpendState,
    },
    organization::OrgRep,
    traits::{
        ConfigureThreshold,
        GetVoteOutcome,
        GroupMembership,
        OpenBankAccount,
        SpendGovernance,
    },
    vote::{
        ThresholdInput,
        VoteOutcome,
        XorThreshold,
    },
};

// type aliases
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type BankSt<T> = BankState<
    <T as Trait>::BankId,
    <T as frame_system::Trait>::AccountId,
    <T as org::Trait>::OrgId,
    <T as vote::Trait>::ThresholdId,
>;
type Threshold<T> = ThresholdInput<
    OrgRep<<T as org::Trait>::OrgId>,
    XorThreshold<<T as vote::Trait>::Signal, Permill>,
>;
type SpendProp<T> = SpendProposal<
    <T as Trait>::BankId,
    <T as Trait>::SpendId,
    BalanceOf<T>,
    <T as frame_system::Trait>::AccountId,
    SpendState<<T as vote::Trait>::VoteId>,
>;

pub trait Trait:
    frame_system::Trait + org::Trait + donate::Trait + vote::Trait
{
    /// The overarching event types
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The base bank account for this module
    type BigBank: Get<ModuleId>;

    /// Identifier for banks
    type BankId: Parameter
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

    /// Identifier for spends
    type SpendId: Parameter
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

    /// Max number of bank accounts for one org
    type MaxTreasuryPerOrg: Get<u32>;
    /// Min to open bank account
    type MinDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BankId,
        <T as Trait>::SpendId,
        Balance = BalanceOf<T>,
    {
        AccountOpened(AccountId, BankId, Balance, OrgId, Option<AccountId>),
        SpendProposed(AccountId, BankId, SpendId, Balance, AccountId),
        VoteTriggered(AccountId, BankId, SpendId, VoteId),
        SudoApproved(AccountId, BankId, SpendId),
        ProposalPolled(BankId, SpendId, SpendState<VoteId>),
        AccountClosed(AccountId, BankId, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        InsufficientBalanceToFundBankOpen,
        CommitteeCountExceedsLimitPerOrg,
        CannotCloseBankThatDNE,
        NotPermittedToOpenBankAccountForOrg,
        NotPermittedToProposeSpendForBankAccount,
        NotPermittedToTriggerVoteForBankAccount,
        NotPermittedToPollSpendProposalForBankAccount,
        CannotSpendIfBankDNE,
        OnlyControllerCanCloseBank,
        OnlyControllerCanSudoApproveSpendProposals,
        // spend proposal stuff
        CannotProposeSpendIfBankDNE,
        BankMustExistToProposeSpendFrom,
        CannotTriggerVoteForSpendIfBaseBankDNE,
        CannotTriggerVoteForSpendIfSpendProposalDNE,
        CannotTriggerVoteFromCurrentSpendProposalState,
        CannotSudoApproveSpendProposalIfBaseBankDNE,
        CannotSudoApproveSpendProposalIfSpendProposalDNE,
        CannotSudoApproveFromCurrentState,
        CannotPollSpendProposalIfBaseBankDNE,
        CannotPollSpendProposalIfSpendProposalDNE,
        // for getting banks for org
        NoBanksForOrg,
        ThresholdCannotBeSetForOrg,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Counter for generating unique bank identifiers
        BankIdNonce get(fn bank_id_nonce): T::BankId;

        /// Counter for generating unique spend proposal identifiers
        SpendNonceMap get(fn spend_nonce_map): map
            hasher(blake2_128_concat) T::BankId => T::SpendId;

        /// Total number of banks registered in this module
        pub TotalBankCount get(fn total_bank_count): u32;

        /// The total number of treasury accounts per org
        pub OrgTreasuryCount get(fn org_treasury_count): map
            hasher(blake2_128_concat) T::OrgId => u32;

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique BankIds
        pub Banks get(fn banks): map
            hasher(blake2_128_concat) T::BankId => Option<BankSt<T>>;

        /// Proposals to make spends from the bank account
        pub SpendProposals get(fn spend_proposals): double_map
            hasher(blake2_128_concat) T::BankId,
            hasher(blake2_128_concat) T::SpendId => Option<SpendProp<T>>;
        /// Frequency for which all spend proposals are polled and pushed along
        SpendPollFrequency get(fn spend_poll_frequency) config(): T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn open(
            origin,
            org: T::OrgId,
            deposit: BalanceOf<T>,
            controller: Option<T::AccountId>,
            threshold: Threshold<T>,
        ) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            ensure!(
                <org::Module<T>>::is_member_of_group(org, &opener),
                Error::<T>::NotPermittedToOpenBankAccountForOrg
            );
            let bank_id = Self::open_bank_account(opener.clone(), org, deposit, controller.clone(), threshold)?;
            Self::deposit_event(RawEvent::AccountOpened(opener, bank_id, deposit, org, controller));
            Ok(())
        }
        #[weight = 0]
        fn propose_spend(
            origin,
            bank_id: T::BankId,
            amount: BalanceOf<T>,
            dest: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let new_spend_id = Self::_propose_spend(&caller, bank_id, amount, dest.clone())?;
            Self::deposit_event(RawEvent::SpendProposed(caller, bank_id, new_spend_id, amount, dest));
            Ok(())
        }
        #[weight = 0]
        fn trigger_vote(
            origin,
            bank_id: T::BankId,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let vote_id = Self::_trigger_vote_on_spend_proposal(&caller, bank_id, spend_id)?;
            Self::deposit_event(RawEvent::VoteTriggered(caller, bank_id, spend_id, vote_id));
            Ok(())
        }
        #[weight = 0]
        fn sudo_approve(
            origin,
            bank_id: T::BankId,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::_sudo_approve_spend_proposal(&caller, bank_id, spend_id)?;
            Self::deposit_event(RawEvent::SudoApproved(caller, bank_id, spend_id));
            Ok(())
        }
        #[weight = 0]
        fn close(
            origin,
            bank_id: T::BankId,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            let bank = <Banks<T>>::get(bank_id).ok_or(Error::<T>::CannotCloseBankThatDNE)?;
            // permissions for closing bank accounts is org supervisor status
            ensure!(
                bank.is_controller(&closer),
                Error::<T>::OnlyControllerCanCloseBank
            );
            let bank_account_id = Self::bank_account_id(bank_id);
            let remaining_funds = <T as donate::Trait>::Currency::total_balance(&bank_account_id);
            // distributes remaining funds equally among members in proportion to ownership (PropDonation)
            let _ = <donate::Module<T>>::donate(
                &bank_account_id,
                OrgRep::Weighted(bank.org()),
                &closer,
                remaining_funds,
            )?;
            <Banks<T>>::remove(bank_id);
            <OrgTreasuryCount<T>>::mutate(bank.org(), |count| *count -= 1);
            <TotalBankCount>::mutate(|count| *count -= 1);
            Self::deposit_event(RawEvent::AccountClosed(closer, bank_id, bank.org()));
            Ok(())
        }
        fn on_finalize(_n: T::BlockNumber) {
            if <frame_system::Module<T>>::block_number() % Self::spend_poll_frequency() == Zero::zero() {
                <SpendProposals<T>>::iter().for_each(|(_, _, prop)| {
                    let (bank_id, spend_id) = (prop.bank_id(), prop.spend_id());
                    if let Ok(state) = Self::poll_spend_proposal(prop) {
                        Self::deposit_event(RawEvent::ProposalPolled(bank_id, spend_id, state));
                    }
                });
            }
        }
    }
}

impl<T: Trait> Module<T> {
    /// Performs computation so don't call unnecessarily
    pub fn bank_account_id(id: T::BankId) -> T::AccountId {
        T::BigBank::get().into_sub_account(id)
    }
    pub fn bank_balance(bank: T::BankId) -> BalanceOf<T> {
        <T as Trait>::Currency::total_balance(&Self::bank_account_id(bank))
    }
    pub fn is_bank(id: T::BankId) -> bool {
        <Banks<T>>::get(id).is_some()
    }
    pub fn is_spend(bank: T::BankId, spend: T::SpendId) -> bool {
        <SpendProposals<T>>::get(bank, spend).is_some()
    }
    fn generate_bank_uid() -> T::BankId {
        let mut bank_nonce_id = <BankIdNonce<T>>::get() + 1u32.into();
        while Self::is_bank(bank_nonce_id) {
            bank_nonce_id += 1u32.into();
        }
        <BankIdNonce<T>>::put(bank_nonce_id);
        bank_nonce_id
    }
    fn generate_spend_uid(seed: T::BankId) -> T::SpendId {
        let mut id_nonce = <SpendNonceMap<T>>::get(seed) + 1u32.into();
        while Self::is_spend(seed, id_nonce) {
            id_nonce += 1u32.into();
        }
        <SpendNonceMap<T>>::insert(seed, id_nonce);
        id_nonce
    }
    pub fn get_banks_for_org(
        org: T::OrgId,
    ) -> Result<Vec<T::BankId>, DispatchError> {
        let ret_vec = <Banks<T>>::iter()
            .filter(|(_, bank_state)| bank_state.org() == org)
            .map(|(bank_id, _)| bank_id)
            .collect::<Vec<T::BankId>>();
        if !ret_vec.is_empty() {
            Ok(ret_vec)
        } else {
            Err(Error::<T>::NoBanksForOrg.into())
        }
    }
}

impl<T: Trait>
    OpenBankAccount<T::OrgId, BalanceOf<T>, T::AccountId, Threshold<T>>
    for Module<T>
{
    type BankId = T::BankId;
    fn open_bank_account(
        opener: T::AccountId,
        org: T::OrgId,
        deposit: BalanceOf<T>,
        controller: Option<T::AccountId>,
        threshold: Threshold<T>,
    ) -> Result<Self::BankId, DispatchError> {
        ensure!(
            deposit >= T::MinDeposit::get(),
            Error::<T>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum
        );
        ensure!(
            <T as Trait>::Currency::free_balance(&opener) > deposit,
            Error::<T>::InsufficientBalanceToFundBankOpen
        );
        let new_count = <OrgTreasuryCount<T>>::get(org) + 1;
        ensure!(
            new_count <= T::MaxTreasuryPerOrg::get(),
            Error::<T>::CommitteeCountExceedsLimitPerOrg
        );
        // TODO: extract into separate method that might compare with min threshold set in this module contex
        ensure!(
            threshold.org().org() == org,
            Error::<T>::ThresholdCannotBeSetForOrg
        );
        // register input threshold
        let threshold_id = <vote::Module<T>>::register_threshold(threshold)?;
        // generate new treasury identifier
        let id = Self::generate_bank_uid();
        // create new bank object
        let bank = BankState::new(id, org, controller, threshold_id);
        // perform fallible transfer
        <T as Trait>::Currency::transfer(
            &opener,
            &Self::bank_account_id(id),
            deposit,
            ExistenceRequirement::KeepAlive,
        )?;
        // insert new bank object
        <Banks<T>>::insert(id, bank);
        // put new org treasury count
        <OrgTreasuryCount<T>>::insert(org, new_count);
        // iterate total bank count
        <TotalBankCount>::mutate(|count| *count += 1u32);
        // return new treasury identifier
        Ok(id)
    }
}

impl<T: Trait>
    SpendGovernance<T::BankId, BalanceOf<T>, T::AccountId, SpendProp<T>>
    for Module<T>
{
    type SpendId = T::SpendId;
    type VoteId = T::VoteId;
    type SpendState = SpendState<T::VoteId>;
    fn _propose_spend(
        caller: &T::AccountId,
        bank_id: T::BankId,
        amount: BalanceOf<T>,
        dest: T::AccountId,
    ) -> Result<Self::SpendId, DispatchError> {
        let bank = <Banks<T>>::get(bank_id)
            .ok_or(Error::<T>::BankMustExistToProposeSpendFrom)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), caller),
            Error::<T>::NotPermittedToProposeSpendForBankAccount
        );
        let id = Self::generate_spend_uid(bank_id);
        let proposal = SpendProposal::new(bank_id, id, amount, dest);
        <SpendProposals<T>>::insert(bank_id, id, proposal);
        Ok(id)
    }
    fn _trigger_vote_on_spend_proposal(
        caller: &T::AccountId,
        bank_id: T::BankId,
        spend_id: Self::SpendId,
    ) -> Result<Self::VoteId, DispatchError> {
        let bank = <Banks<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotTriggerVoteForSpendIfBaseBankDNE)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), caller),
            Error::<T>::NotPermittedToTriggerVoteForBankAccount
        );
        let spend_proposal = <SpendProposals<T>>::get(bank_id, spend_id)
            .ok_or(Error::<T>::CannotTriggerVoteForSpendIfSpendProposalDNE)?;
        match spend_proposal.state() {
            SpendState::WaitingForApproval => {
                // dispatch vote with bank's default threshold
                let new_vote_id = <vote::Module<T>>::invoke_threshold(
                    bank.threshold_id(),
                    None, // TODO: use vote info ref here instead of None
                    None,
                )?;
                let new_spend_proposal =
                    spend_proposal.set_state(SpendState::Voting(new_vote_id));
                <SpendProposals<T>>::insert(
                    bank_id,
                    spend_id,
                    new_spend_proposal,
                );
                Ok(new_vote_id)
            }
            _ => {
                Err(Error::<T>::CannotTriggerVoteFromCurrentSpendProposalState
                    .into())
            }
        }
    }
    fn _sudo_approve_spend_proposal(
        caller: &T::AccountId,
        bank_id: T::BankId,
        spend_id: Self::SpendId,
    ) -> DispatchResult {
        let bank = <Banks<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSudoApproveSpendProposalIfBaseBankDNE)?;
        ensure!(
            bank.is_controller(caller),
            Error::<T>::OnlyControllerCanSudoApproveSpendProposals
        );
        let spend_proposal = <SpendProposals<T>>::get(bank_id, spend_id)
            .ok_or(
                Error::<T>::CannotSudoApproveSpendProposalIfSpendProposalDNE,
            )?;
        match spend_proposal.state() {
            SpendState::WaitingForApproval => {
                // TODO: if Voting, remove the current live vote
                let new_spend_proposal = if let Ok(()) =
                    <T as Trait>::Currency::transfer(
                        &Self::bank_account_id(bank_id),
                        &spend_proposal.dest(),
                        spend_proposal.amount(),
                        ExistenceRequirement::KeepAlive,
                    ) {
                    spend_proposal.set_state(SpendState::ApprovedAndExecuted)
                } else {
                    spend_proposal.set_state(SpendState::ApprovedButNotExecuted)
                };
                <SpendProposals<T>>::insert(
                    bank_id,
                    spend_id,
                    new_spend_proposal,
                );
                Ok(())
            }
            _ => Err(Error::<T>::CannotSudoApproveFromCurrentState.into()),
        }
    }
    fn poll_spend_proposal(
        prop: SpendProp<T>,
    ) -> Result<Self::SpendState, DispatchError> {
        ensure!(
            Self::is_bank(prop.bank_id()),
            Error::<T>::CannotPollSpendProposalIfBaseBankDNE
        );
        let _ = <SpendProposals<T>>::get(prop.bank_id(), prop.spend_id())
            .ok_or(Error::<T>::CannotPollSpendProposalIfSpendProposalDNE)?;
        match prop.state() {
            SpendState::Voting(vote_id) => {
                let vote_outcome =
                    <vote::Module<T>>::get_vote_outcome(vote_id)?;
                if vote_outcome == VoteOutcome::Approved {
                    // approved so try to execute and if not, still approve
                    let new_spend_proposal = if let Ok(()) =
                        <T as Trait>::Currency::transfer(
                            &Self::bank_account_id(prop.bank_id()),
                            &prop.dest(),
                            prop.amount(),
                            ExistenceRequirement::KeepAlive,
                        ) {
                        prop.set_state(SpendState::ApprovedAndExecuted)
                    } else {
                        prop.set_state(SpendState::ApprovedButNotExecuted)
                    };
                    let ret_state = new_spend_proposal.state();
                    <SpendProposals<T>>::insert(
                        prop.bank_id(),
                        prop.spend_id(),
                        new_spend_proposal,
                    );
                    Ok(ret_state)
                } else {
                    Ok(prop.state())
                }
            }
            _ => Ok(prop.state()),
        }
    }
}
