#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Bank account for orgs w/ democratic escrow rules

#[cfg(test)]
mod tests;

use codec::Codec;
use frame_support::{
    decl_error,
    decl_event,
    decl_module,
    decl_storage,
    ensure,
    storage::IterableStorageMap,
    traits::{
        Currency,
        ExistenceRequirement,
        Get,
        ReservableCurrency,
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
        BankSpend,
        BankState,
        OnChainTreasuryID,
        SpendProposal,
        SpendState,
    },
    traits::{
        BankPermissions,
        GenerateUniqueID,
        GetVoteOutcome,
        GroupMembership,
        IDIsAvailable,
        Increment,
        OpenBankAccount,
        OpenVote,
        OrganizationSupervisorPermissions,
        SeededGenerateUniqueID,
        SpendGovernance,
    },
    vote::VoteOutcome,
};

/// The balances type for this module
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + org::Trait + donate::Trait + vote::Trait
{
    /// The overarching event types
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Identifier for spends, only has meaning in the context of a bank account (OnChainTreasuryID)
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

    /// The currency type for on-chain transactions
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The limit on how many bank accounts an org can have
    type MaxTreasuryPerOrg: Get<u32>;

    /// The minimum amount necessary to open an organizational bank account
    type MinimumInitialDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as vote::Trait>::VoteId,
        <T as Trait>::SpendId,
        Balance = BalanceOf<T>,
    {
        BankAccountOpened(AccountId, OnChainTreasuryID, Balance, OrgId, Option<AccountId>),
        SpendProposedByMember(AccountId, OnChainTreasuryID, SpendId, Balance, AccountId),
        VoteTriggeredOnSpendProposal(AccountId, OnChainTreasuryID, SpendId, VoteId),
        SudoApprovedSpendProposal(AccountId, OnChainTreasuryID, SpendId),
        SpendProposalPolled(AccountId, OnChainTreasuryID, SpendId, SpendState<VoteId>),
        BankAccountClosed(AccountId, OnChainTreasuryID, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg,
        CannotCloseBankThatDNE,
        NotPermittedToOpenBankAccountForOrg,
        NotPermittedToProposeSpendForBankAccount,
        NotPermittedToTriggerVoteForBankAccount,
        NotPermittedToSudoApproveForBankAccount,
        NotPermittedToPollSpendProposalForBankAccount,
        CannotSpendIfBankDNE,
        MustBeOrgSupervisorToCloseBankAccount,
        // spend proposal stuff
        CannotProposeSpendIfBankDNE,
        BankMustExistToProposeSpendFrom,
        CannotTriggerVoteForSpendIfBaseBankDNE,
        CannotTriggerVoteForSpendIfSpendProposalDNE,
        CannotTriggerVoteFromCurrentSpendProposalState,
        CannotSudoApproveSpendProposalIfBaseBankDNE,
        CannotSudoApproveSpendProposalIfSpendProposalDNE,
        CannotApproveAlreadyApprovedSpendProposal,
        CannotPollSpendProposalIfBaseBankDNE,
        CannotPollSpendProposalIfSpendProposalDNE,
        // for getting banks for org
        NoBanksForOrg,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Counter for generating unique treasury identifiers
        TreasuryIDNonce get(fn treasury_id_nonce): OnChainTreasuryID;

        /// Counter for generating unique spend proposal identifiers
        SpendNonceMap get(fn spend_nonce_map): map
            hasher(blake2_128_concat) OnChainTreasuryID => T::SpendId;

        /// Total number of banks registered in this module
        pub TotalBankCount get(fn total_bank_count): u32;

        /// The total number of treasury accounts per org
        pub OrgTreasuryCount get(fn org_treasury_count): map
            hasher(blake2_128_concat) T::OrgId => u32;

        /// The store for organizational bank accounts
        /// -> keyset acts as canonical set for unique `OnChainTreasuryID`s
        pub BankStores get(fn bank_stores): map
            hasher(blake2_128_concat) OnChainTreasuryID =>
            Option<BankState<T::AccountId, T::OrgId>>;

        /// Proposals to make spends from the bank account
        /// TODO: gc strategy in `on_finalize`
        pub SpendProposals get(fn spend_proposals): double_map
            hasher(blake2_128_concat) OnChainTreasuryID,
            hasher(blake2_128_concat) T::SpendId => Option<
                SpendProposal<
                    BalanceOf<T>,
                    T::AccountId,
                    SpendState<T::VoteId>
                >
            >;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn open_org_bank_account(
            origin,
            org: T::OrgId,
            deposit: BalanceOf<T>,
            controller: Option<T::AccountId>,
        ) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            let auth = Self::can_open_bank_account_for_org(org, &opener);
            ensure!(auth, Error::<T>::NotPermittedToOpenBankAccountForOrg);
            let bank_id = Self::open_bank_account(opener.clone(), org, deposit, controller.clone())?;
            Self::deposit_event(RawEvent::BankAccountOpened(opener, bank_id, deposit, org, controller));
            Ok(())
        }
        #[weight = 0]
        fn member_proposes_spend(
            origin,
            bank_id: OnChainTreasuryID,
            amount: BalanceOf<T>,
            dest: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_propose_spend(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToProposeSpendForBankAccount);
            let new_spend_id = Self::propose_spend(bank_id, amount, dest.clone())?;
            Self::deposit_event(RawEvent::SpendProposedByMember(caller, bank_id, new_spend_id.spend, amount, dest));
            Ok(())
        }
        #[weight = 0]
        fn member_triggers_vote_on_spend_proposal(
            origin,
            bank_id: OnChainTreasuryID,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_trigger_vote_on_spend_proposal(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToTriggerVoteForBankAccount);
            let bank_spend_id = BankSpend::new(bank_id, spend_id);
            let vote_id = Self::trigger_vote_on_spend_proposal(bank_spend_id)?;
            Self::deposit_event(RawEvent::VoteTriggeredOnSpendProposal(caller, bank_id, spend_id, vote_id));
            Ok(())
        }
        #[weight = 0]
        fn member_sudo_approves_spend_proposal(
            origin,
            bank_id: OnChainTreasuryID,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_sudo_approve_spend_proposal(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToSudoApproveForBankAccount);
            let bank_spend_id = BankSpend::new(bank_id, spend_id);
            Self::sudo_approve_spend_proposal(bank_spend_id)?;
            Self::deposit_event(RawEvent::SudoApprovedSpendProposal(caller, bank_id, spend_id));
            Ok(())
        }
        #[weight = 0]
        fn member_polls_spend_proposal(
            origin,
            bank_id: OnChainTreasuryID,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let auth = Self::can_poll_spend_proposal(bank_id, &caller)?;
            ensure!(auth, Error::<T>::NotPermittedToPollSpendProposalForBankAccount);
            let bank_spend_id = BankSpend::new(bank_id, spend_id);
            let state = Self::poll_spend_proposal(bank_spend_id)?;
            Self::deposit_event(RawEvent::SpendProposalPolled(caller, bank_id, spend_id, state));
            Ok(())
        }
        #[weight = 0]
        fn close_org_bank_account(
            origin,
            bank_id: OnChainTreasuryID,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            let bank = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::CannotCloseBankThatDNE)?;
            // permissions for closing bank accounts is org supervisor status
            ensure!(
                <org::Module<T>>::is_organization_supervisor(bank.org(), &closer),
                Error::<T>::MustBeOrgSupervisorToCloseBankAccount
            );
            let bank_account_id = Self::account_id(bank_id);
            let remaining_funds = <T as donate::Trait>::Currency::total_balance(&bank_account_id);
            // distributes remaining funds equally among members
            <donate::Module<T>>::donate(
                &bank_account_id,
                bank.org(),
                remaining_funds,
                false,
            )?;
            <BankStores<T>>::remove(bank_id);
            <OrgTreasuryCount<T>>::mutate(bank.org(), |count| *count -= 1);
            <TotalBankCount>::mutate(|count| *count -= 1);
            Self::deposit_event(RawEvent::BankAccountClosed(closer, bank_id, bank.org()));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn is_bank(id: OnChainTreasuryID) -> bool {
        !Self::id_is_available(id)
    }
    pub fn account_id(id: OnChainTreasuryID) -> T::AccountId {
        id.into_account()
    }
    pub fn bank_balance(bank: OnChainTreasuryID) -> BalanceOf<T> {
        <T as Trait>::Currency::total_balance(&Self::account_id(bank))
    }
    pub fn get_banks_for_org(
        org: T::OrgId,
    ) -> Result<Vec<OnChainTreasuryID>, DispatchError> {
        let ret_vec = <BankStores<T>>::iter()
            .filter(|(_, bank_state)| bank_state.org() == org)
            .map(|(bank_id, _)| bank_id)
            .collect::<Vec<OnChainTreasuryID>>();
        if !ret_vec.is_empty() {
            Ok(ret_vec)
        } else {
            Err(Error::<T>::NoBanksForOrg.into())
        }
    }
}

impl<T: Trait> IDIsAvailable<OnChainTreasuryID> for Module<T> {
    fn id_is_available(id: OnChainTreasuryID) -> bool {
        <BankStores<T>>::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(OnChainTreasuryID, T::SpendId)> for Module<T> {
    fn id_is_available(id: (OnChainTreasuryID, T::SpendId)) -> bool {
        <SpendProposals<T>>::get(id.0, id.1).is_none()
    }
}

impl<T: Trait> GenerateUniqueID<OnChainTreasuryID> for Module<T> {
    fn generate_unique_id() -> OnChainTreasuryID {
        let mut treasury_nonce_id = TreasuryIDNonce::get().increment();
        while !Self::id_is_available(treasury_nonce_id) {
            treasury_nonce_id = treasury_nonce_id.increment();
        }
        TreasuryIDNonce::put(treasury_nonce_id);
        treasury_nonce_id
    }
}

impl<T: Trait> SeededGenerateUniqueID<T::SpendId, OnChainTreasuryID>
    for Module<T>
{
    fn seeded_generate_unique_id(seed: OnChainTreasuryID) -> T::SpendId {
        let mut id_nonce = <SpendNonceMap<T>>::get(seed) + 1u32.into();
        while !Self::id_is_available((seed, id_nonce)) {
            id_nonce += 1u32.into();
        }
        <SpendNonceMap<T>>::insert(seed, id_nonce);
        id_nonce
    }
}

// adds a storage lookup /forall methods but this is just for local permissions anyway
impl<T: Trait> BankPermissions<OnChainTreasuryID, T::OrgId, T::AccountId>
    for Module<T>
{
    fn can_open_bank_account_for_org(
        org: T::OrgId,
        who: &T::AccountId,
    ) -> bool {
        <org::Module<T>>::is_member_of_group(org, who)
    }
    fn can_propose_spend(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::CannotProposeSpendIfBankDNE)?;
        Ok(<org::Module<T>>::is_member_of_group(bank.org(), who))
    }
    fn can_trigger_vote_on_spend_proposal(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::CannotTriggerVoteForSpendIfBaseBankDNE)?;
        Ok(<org::Module<T>>::is_member_of_group(bank.org(), who))
    }
    fn can_sudo_approve_spend_proposal(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::CannotSudoApproveSpendProposalIfBaseBankDNE)?;
        Ok(bank.is_controller(who))
    }
    fn can_poll_spend_proposal(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::CannotPollSpendProposalIfBaseBankDNE)?;
        Ok(<org::Module<T>>::is_member_of_group(bank.org(), who))
    }
    fn can_spend(
        bank: OnChainTreasuryID,
        who: &T::AccountId,
    ) -> Result<bool, DispatchError> {
        let bank = <BankStores<T>>::get(bank)
            .ok_or(Error::<T>::CannotSpendIfBankDNE)?;
        Ok(bank.is_controller(who))
    }
}

impl<T: Trait> OpenBankAccount<T::OrgId, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    type BankId = OnChainTreasuryID;
    fn open_bank_account(
        opener: T::AccountId,
        org: T::OrgId,
        deposit: BalanceOf<T>,
        controller: Option<T::AccountId>,
    ) -> Result<Self::BankId, DispatchError> {
        ensure!(
            deposit >= T::MinimumInitialDeposit::get(),
            Error::<T>::CannotOpenBankAccountIfDepositIsBelowModuleMinimum
        );
        let new_org_bank_count = <OrgTreasuryCount<T>>::get(org) + 1;
        ensure!(
            new_org_bank_count <= T::MaxTreasuryPerOrg::get(),
            Error::<T>::CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg
        );
        // generate new treasury identifier
        let new_treasury_id = Self::generate_unique_id();
        // create new bank object
        let new_bank = BankState::new(org, controller.clone());
        // perform fallible transfer
        <T as Trait>::Currency::transfer(
            &opener,
            &Self::account_id(new_treasury_id),
            deposit,
            ExistenceRequirement::KeepAlive,
        )?;
        // insert new bank object
        <BankStores<T>>::insert(new_treasury_id, new_bank);
        // iterate org treasury count
        <OrgTreasuryCount<T>>::insert(org, new_org_bank_count);
        // iterate total bank count
        <TotalBankCount>::mutate(|count| *count += 1u32);
        // return new treasury identifier
        Ok(new_treasury_id)
    }
}

impl<T: Trait> SpendGovernance<OnChainTreasuryID, BalanceOf<T>, T::AccountId>
    for Module<T>
{
    type SpendId = BankSpend<OnChainTreasuryID, T::SpendId>;
    type VoteId = T::VoteId;
    type SpendState = SpendState<T::VoteId>;
    fn propose_spend(
        bank_id: OnChainTreasuryID,
        amount: BalanceOf<T>,
        dest: T::AccountId,
    ) -> Result<Self::SpendId, DispatchError> {
        ensure!(
            Self::is_bank(bank_id),
            Error::<T>::BankMustExistToProposeSpendFrom
        );
        let spend_proposal = SpendProposal::new(amount, dest);
        let new_spend_id = Self::seeded_generate_unique_id(bank_id);
        <SpendProposals<T>>::insert(bank_id, new_spend_id, spend_proposal);
        Ok(BankSpend::new(bank_id, new_spend_id))
    }
    fn trigger_vote_on_spend_proposal(
        spend_id: Self::SpendId,
    ) -> Result<Self::VoteId, DispatchError> {
        let bank = <BankStores<T>>::get(spend_id.bank)
            .ok_or(Error::<T>::CannotTriggerVoteForSpendIfBaseBankDNE)?;
        let spend_proposal =
            <SpendProposals<T>>::get(spend_id.bank, spend_id.spend).ok_or(
                Error::<T>::CannotTriggerVoteForSpendIfSpendProposalDNE,
            )?;
        match spend_proposal.state() {
            SpendState::WaitingForApproval => {
                // default unanimous passage \forall spend proposals; TODO: add more default thresholds after more user research and consider adding local storage item for the threshold
                let new_vote_id = <vote::Module<T>>::open_unanimous_consent(
                    None,
                    bank.org(),
                    None,
                )?;
                let new_spend_proposal =
                    spend_proposal.set_state(SpendState::Voting(new_vote_id));
                <SpendProposals<T>>::insert(
                    spend_id.bank,
                    spend_id.spend,
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
    fn sudo_approve_spend_proposal(spend_id: Self::SpendId) -> DispatchResult {
        ensure!(
            Self::is_bank(spend_id.bank),
            Error::<T>::CannotSudoApproveSpendProposalIfBaseBankDNE
        );
        let spend_proposal =
            <SpendProposals<T>>::get(spend_id.bank, spend_id.spend).ok_or(
                Error::<T>::CannotSudoApproveSpendProposalIfSpendProposalDNE,
            )?;
        match spend_proposal.state() {
            SpendState::WaitingForApproval | SpendState::Voting(_) => {
                // TODO: if Voting, remove the current live vote
                let new_spend_proposal = if let Ok(()) =
                    <T as Trait>::Currency::transfer(
                        &Self::account_id(spend_id.bank),
                        &spend_proposal.dest(),
                        spend_proposal.amount(),
                        ExistenceRequirement::KeepAlive,
                    ) {
                    spend_proposal.set_state(SpendState::ApprovedAndExecuted)
                } else {
                    spend_proposal.set_state(SpendState::ApprovedButNotExecuted)
                };
                <SpendProposals<T>>::insert(
                    spend_id.bank,
                    spend_id.spend,
                    new_spend_proposal,
                );
                Ok(())
            }
            _ => {
                Err(Error::<T>::CannotApproveAlreadyApprovedSpendProposal
                    .into())
            }
        }
    }
    fn poll_spend_proposal(
        spend_id: Self::SpendId,
    ) -> Result<Self::SpendState, DispatchError> {
        ensure!(
            Self::is_bank(spend_id.bank),
            Error::<T>::CannotPollSpendProposalIfBaseBankDNE
        );
        let spend_proposal =
            <SpendProposals<T>>::get(spend_id.bank, spend_id.spend)
                .ok_or(Error::<T>::CannotPollSpendProposalIfSpendProposalDNE)?;
        match spend_proposal.state() {
            SpendState::Voting(vote_id) => {
                let vote_outcome =
                    <vote::Module<T>>::get_vote_outcome(vote_id)?;
                if vote_outcome == VoteOutcome::Approved {
                    // approved so try to execute and if not, still approve
                    let new_spend_proposal = if let Ok(()) =
                        <T as Trait>::Currency::transfer(
                            &Self::account_id(spend_id.bank),
                            &spend_proposal.dest(),
                            spend_proposal.amount(),
                            ExistenceRequirement::KeepAlive,
                        ) {
                        spend_proposal
                            .set_state(SpendState::ApprovedAndExecuted)
                    } else {
                        spend_proposal
                            .set_state(SpendState::ApprovedButNotExecuted)
                    };
                    let ret_state = new_spend_proposal.state();
                    <SpendProposals<T>>::insert(
                        spend_id.bank,
                        spend_id.spend,
                        new_spend_proposal.clone(),
                    );
                    Ok(ret_state)
                } else {
                    Ok(spend_proposal.state())
                }
            }
            _ => Ok(spend_proposal.state()),
        }
    }
}
