#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Moloch impl

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
    moloch::{
        MembershipProposal,
        ProposalState,
    },
    organization::OrgRep,
    traits::{
        ConfigureThreshold,
        GetVoteOutcome,
        GroupMembership,
        MolochMembership,
        OpenBankAccount,
        ShareIssuance,
        SpendGovernance,
    },
    vote::{
        ThresholdConfig,
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
type Threshold<T> = ThresholdConfig<
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
type MemberProp<T> = MembershipProposal<
    <T as Trait>::BankId,
    <T as Trait>::ProposalId,
    BalanceOf<T>,
    <T as org::Trait>::Shares,
    <T as frame_system::Trait>::AccountId,
    ProposalState<<T as vote::Trait>::VoteId>,
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

    /// Identifier for spend proposals
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

    /// Identifier for membership proposals
    type ProposalId: Parameter
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

    /// The minimum amount to open an organizational bank account and keep it open
    type MinDeposit: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::Shares,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BankId,
        <T as Trait>::SpendId,
        <T as Trait>::ProposalId,
        Balance = BalanceOf<T>,
    {
        BankAccountOpened(AccountId, BankId, Balance, OrgId, Option<AccountId>),
        NewMemberProposal(AccountId, BankId, ProposalId, Balance, Shares, AccountId),
        VoteTriggeredOnMemberProposal(AccountId, BankId, ProposalId, VoteId),
        SpendProposedByMember(AccountId, BankId, SpendId, Balance, AccountId),
        VoteTriggeredOnSpendProposal(AccountId, BankId, SpendId, VoteId),
        SudoApprovedSpendProposal(AccountId, BankId, SpendId),
        SpendProposalPolled(BankId, SpendId, SpendState<VoteId>),
        MemberProposalPolled(BankId, ProposalId, ProposalState<VoteId>),
        // relevant org and number of shares burned
        SharesBurned(OrgId, Shares),
        // bank, amt withdrawn by burn, amt left in bank
        WithdrawnPortion(BankId, Balance, Balance),
        BankAccountClosed(AccountId, BankId, OrgId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        LimitOfOneMolochPerOrg,
        InsufficientBalanceToFundBankOpen,
        CannotOpenBankAccountIfDepositIsBelowModuleMinimum,
        CannotOpenBankAccountForOrgIfBankCountExceedsLimitPerOrg,
        CannotCloseBankThatDNE,
        NotPermittedToOpenBankAccountForOrg,
        NotPermittedToTriggerVoteForBankAccount,
        NotPermittedToSudoApproveForBankAccount,
        NotPermittedToPollProposalForBankAccount,
        CannotSpendIfBankDNE,
        MustBeOrgSupervisorToCloseBankAccount,
        // shared proposal errs
        CannotProposeIfBankDNE,
        BankMustExistToProposeFrom,
        CannotTriggerVoteIfBaseBankDNE,
        CannotTriggerVoteIfProposalDNE,
        MustBeMemberToSponsorProposal,
        // spend proposal errs
        CannotTriggerVoteFromCurrentSpendProposalState,
        CannotSudoApproveSpendProposalIfBaseBankDNE,
        CannotSudoApproveSpendProposalIfSpendProposalDNE,
        CannotApproveAlreadyApprovedSpendProposal,
        CannotPollProposalIfBaseBankDNE,
        CannotPollProposalIfProposalDNE,
        // moloch member errs
        CannotTriggerVoteFromCurrentMemberProposalState,
        CannotBurnSharesIfBaseBankDNE,
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

        /// Counter for generating unique membership proposal identifiers
        ProposalNonceMap get(fn proposal_nonce_map): map
            hasher(blake2_128_concat) T::BankId => T::ProposalId;

        /// Total number of banks registered in this module
        pub TotalBankCount get(fn total_bank_count): u32;
        /// Hashset of orgs that have bank accounts
        pub OrgBankRegistrar get(fn org_bank_registrar): map
            hasher(blake2_128_concat) T::OrgId => Option<()>;

        /// The store for organizational bank accounts
        pub BankStores get(fn bank_stores): map
            hasher(blake2_128_concat) T::BankId => Option<BankSt<T>>;

        /// Proposals to make spends from the bank account
        pub SpendProps get(fn spend_props): double_map
            hasher(blake2_128_concat) T::BankId,
            hasher(blake2_128_concat) T::SpendId => Option<SpendProp<T>>;

        /// Proposals to join the membership of the bank
        pub MemberProps get(fn member_props): double_map
            hasher(blake2_128_concat) T::BankId,
            hasher(blake2_128_concat) T::ProposalId => Option<MemberProp<T>>;

        /// Frequency for which all spend proposals are polled and pushed along
        SpendPollFrequency get(fn spend_poll_frequency) config(): T::BlockNumber;
        /// Frequency for which all membership proposals are polled and pushed along
        MemberPollFrequency get(fn member_poll_frequency) config(): T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn summon(
            origin,
            org: T::OrgId,
            deposit: BalanceOf<T>,
            controller: Option<T::AccountId>,
            threshold: Threshold<T>,
        ) -> DispatchResult {
            ensure!(<OrgBankRegistrar<T>>::get(org).is_none(), Error::<T>::LimitOfOneMolochPerOrg);
            let opener = ensure_signed(origin)?;
            ensure!(
                <org::Module<T>>::is_member_of_group(org, &opener),
                Error::<T>::NotPermittedToOpenBankAccountForOrg
            );
            let bank_id = Self::open_bank_account(opener.clone(), org, deposit, controller.clone(), threshold)?;
            <OrgBankRegistrar<T>>::insert(org, ());
            Self::deposit_event(RawEvent::BankAccountOpened(opener, bank_id, deposit, org, controller));
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
            Self::deposit_event(RawEvent::SpendProposedByMember(caller, bank_id, new_spend_id, amount, dest));
            Ok(())
        }
        #[weight = 0]
        fn trigger_vote_on_spend_proposal(
            origin,
            bank_id: T::BankId,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let vote_id = Self::_trigger_vote_on_spend_proposal(&caller, bank_id, spend_id)?;
            Self::deposit_event(RawEvent::VoteTriggeredOnSpendProposal(caller, bank_id, spend_id, vote_id));
            Ok(())
        }
        #[weight = 0]
        fn sudo_approve_spend_proposal(
            origin,
            bank_id: T::BankId,
            spend_id: T::SpendId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::_sudo_approve_spend_proposal(&caller, bank_id, spend_id)?;
            Self::deposit_event(RawEvent::SudoApprovedSpendProposal(caller, bank_id, spend_id));
            Ok(())
        }
        #[weight = 0]
        fn propose_member(
            origin,
            bank_id: T::BankId,
            tribute: BalanceOf<T>,
            shares_requested: T::Shares,
            applicant: T::AccountId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let proposal_id = Self::_propose_member(&caller, bank_id, tribute, shares_requested, applicant.clone())?;
            Self::deposit_event(RawEvent::NewMemberProposal(caller, bank_id, proposal_id, tribute, shares_requested, applicant));
            Ok(())
        }
        #[weight = 0]
        fn trigger_vote_on_member_proposal(
            origin,
            bank_id: T::BankId,
            proposal_id: T::ProposalId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let new_vote_id = Self::_trigger_vote_on_member_proposal(&caller, bank_id, proposal_id)?;
            Self::deposit_event(RawEvent::VoteTriggeredOnMemberProposal(caller, bank_id, proposal_id, new_vote_id));
            Ok(())
        }
        #[weight = 0]
        fn burn_shares(
            origin,
            bank_id: T::BankId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::_burn_shares(caller, bank_id)?;
            Ok(())
        }
        #[weight = 0]
        fn close_org_bank_account(
            origin,
            bank_id: T::BankId,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            let bank = <BankStores<T>>::get(bank_id).ok_or(Error::<T>::CannotCloseBankThatDNE)?;
            ensure!(
                bank.is_controller(&closer),
                Error::<T>::MustBeOrgSupervisorToCloseBankAccount
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
            <BankStores<T>>::remove(bank_id);
            <TotalBankCount>::mutate(|count| *count -= 1);
            <OrgBankRegistrar<T>>::remove(bank.org());
            Self::deposit_event(RawEvent::BankAccountClosed(closer, bank_id, bank.org()));
            Ok(())
        }
        fn on_finalize(_n: T::BlockNumber) {
            if <frame_system::Module<T>>::block_number() % Self::spend_poll_frequency() == Zero::zero() {
                <SpendProps<T>>::iter().for_each(|(_, _, prop)| {
                    let (bank_id, spend_id) = (prop.bank_id(), prop.spend_id());
                    if let Ok(state) = Self::poll_spend_proposal(prop) {
                        Self::deposit_event(RawEvent::SpendProposalPolled(bank_id, spend_id, state));
                    }
                });
            }
            if <frame_system::Module<T>>::block_number() % Self::member_poll_frequency() == Zero::zero() {
                <MemberProps<T>>::iter().for_each(|(_, _, prop)| {
                    let (bank_id, prop_id) = (prop.bank_id(), prop.prop_id());
                    if let Ok(state) = Self::poll_membership_proposal(prop) {
                        Self::deposit_event(RawEvent::MemberProposalPolled(bank_id, prop_id, state));
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
        <BankStores<T>>::get(id).is_some()
    }
    pub fn is_spend(bank: T::BankId, spend: T::SpendId) -> bool {
        <SpendProps<T>>::get(bank, spend).is_some()
    }
    pub fn is_proposal(bank: T::BankId, proposal: T::ProposalId) -> bool {
        <MemberProps<T>>::get(bank, proposal).is_some()
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
    fn generate_proposal_uid(seed: T::BankId) -> T::ProposalId {
        let mut id_nonce = <ProposalNonceMap<T>>::get(seed) + 1u32.into();
        while Self::is_proposal(seed, id_nonce) {
            id_nonce += 1u32.into();
        }
        <ProposalNonceMap<T>>::insert(seed, id_nonce);
        id_nonce
    }
    pub fn get_org_bank(org: T::OrgId) -> Result<T::BankId, DispatchError> {
        let mut ret = <BankStores<T>>::iter()
            .filter(|(_, bank_state)| bank_state.org() == org)
            .map(|(bank_id, _)| bank_id)
            .collect::<Vec<T::BankId>>();
        if !ret.is_empty() {
            Ok(ret
                .pop()
                .expect("just checked len > 0 to enter if branch; qed"))
        } else {
            Err(Error::<T>::NoBanksForOrg.into())
        }
    }
}

// // Helper runtime storage method
impl<T: Trait> Module<T> {
    fn execute_member_proposal(
        bank: BankSt<T>,
        applicant: T::AccountId,
        tribute: BalanceOf<T>,
        shares_to_mint: T::Shares,
    ) -> DispatchResult {
        // transfer the tribute from the applicant to the bank
        <T as Trait>::Currency::transfer(
            &applicant,
            &Self::bank_account_id(bank.id()),
            tribute,
            ExistenceRequirement::KeepAlive,
        )?;
        // mint shares in bank.org() for the applicant
        <org::Module<T>>::issue(
            bank.org(),
            applicant,
            shares_to_mint,
            false, // not batch issuance
        )?;
        Ok(())
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
        // TODO: extract into separate method that might compare with min threshold(s) set in this module context
        ensure!(
            threshold.org().org() == org,
            Error::<T>::ThresholdCannotBeSetForOrg
        );
        // register input threshold
        let threshold_id = <vote::Module<T>>::register_threshold(threshold)?;
        // generate new moloch bank identifier
        let id = Self::generate_bank_uid();
        // perform fallible transfer
        <T as Trait>::Currency::transfer(
            &opener,
            &Self::bank_account_id(id),
            deposit,
            ExistenceRequirement::KeepAlive,
        )?;
        // create new bank object
        let new_bank = BankState::new(id, org, controller, threshold_id);
        // insert new bank object
        <BankStores<T>>::insert(id, new_bank);
        // iterate total bank count
        <TotalBankCount>::mutate(|count| *count += 1u32);
        // return new moloch bank identifier
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
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankMustExistToProposeFrom)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), &caller),
            Error::<T>::MustBeMemberToSponsorProposal
        );
        let new_spend_id = Self::generate_spend_uid(bank_id);
        let spend_proposal =
            SpendProp::<T>::new(bank_id, new_spend_id, amount, dest);
        <SpendProps<T>>::insert(bank_id, new_spend_id, spend_proposal);
        Ok(new_spend_id)
    }
    fn _trigger_vote_on_spend_proposal(
        caller: &T::AccountId,
        bank_id: T::BankId,
        spend_id: Self::SpendId,
    ) -> Result<Self::VoteId, DispatchError> {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotTriggerVoteIfBaseBankDNE)?;
        let spend_proposal = <SpendProps<T>>::get(bank_id, spend_id)
            .ok_or(Error::<T>::CannotTriggerVoteIfProposalDNE)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), caller),
            Error::<T>::NotPermittedToTriggerVoteForBankAccount
        );
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
                <SpendProps<T>>::insert(bank_id, spend_id, new_spend_proposal);
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
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotSudoApproveSpendProposalIfBaseBankDNE)?;
        ensure!(
            bank.is_controller(caller),
            Error::<T>::NotPermittedToSudoApproveForBankAccount
        );
        let spend_proposal = <SpendProps<T>>::get(bank_id, spend_id).ok_or(
            Error::<T>::CannotSudoApproveSpendProposalIfSpendProposalDNE,
        )?;
        match spend_proposal.state() {
            SpendState::WaitingForApproval | SpendState::Voting(_) => {
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
                <SpendProps<T>>::insert(bank_id, spend_id, new_spend_proposal);
                Ok(())
            }
            _ => {
                Err(Error::<T>::CannotApproveAlreadyApprovedSpendProposal
                    .into())
            }
        }
    }
    fn poll_spend_proposal(
        prop: SpendProp<T>,
    ) -> Result<Self::SpendState, DispatchError> {
        let _ = <BankStores<T>>::get(prop.bank_id())
            .ok_or(Error::<T>::CannotPollProposalIfBaseBankDNE)?;
        let _ = <SpendProps<T>>::get(prop.bank_id(), prop.spend_id())
            .ok_or(Error::<T>::CannotPollProposalIfProposalDNE)?;
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
                    <SpendProps<T>>::insert(
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

impl<T: Trait>
    MolochMembership<
        T::AccountId,
        T::BankId,
        BalanceOf<T>,
        T::Shares,
        MemberProp<T>,
    > for Module<T>
{
    type MemberPropId = T::ProposalId;
    type VoteId = T::VoteId;
    type PropState = ProposalState<T::VoteId>;
    fn _propose_member(
        caller: &T::AccountId,
        bank_id: T::BankId,
        tribute: BalanceOf<T>,
        shares_requested: T::Shares,
        applicant: T::AccountId,
    ) -> Result<Self::MemberPropId, DispatchError> {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::BankMustExistToProposeFrom)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), &caller),
            Error::<T>::MustBeMemberToSponsorProposal
        );
        let id = Self::generate_proposal_uid(bank_id);
        let member_proposal = MemberProp::<T>::new(
            bank_id,
            id,
            tribute,
            shares_requested,
            applicant,
        );
        <MemberProps<T>>::insert(bank_id, id, member_proposal);
        Ok(id)
    }
    fn _trigger_vote_on_member_proposal(
        caller: &T::AccountId,
        bank_id: T::BankId,
        proposal_id: Self::MemberPropId,
    ) -> Result<Self::VoteId, DispatchError> {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotTriggerVoteIfBaseBankDNE)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(bank.org(), &caller),
            Error::<T>::NotPermittedToTriggerVoteForBankAccount
        );
        let member_proposal = <MemberProps<T>>::get(bank_id, proposal_id)
            .ok_or(Error::<T>::CannotTriggerVoteIfProposalDNE)?;
        match member_proposal.state() {
            ProposalState::WaitingForApproval => {
                // dispatch vote with bank's default threshold
                let new_vote_id = <vote::Module<T>>::invoke_threshold(
                    bank.threshold_id(),
                    None, // TODO: use vote info ref here instead of None
                    None,
                )?;
                let new_member_proposal = member_proposal
                    .set_state(ProposalState::Voting(new_vote_id));
                <MemberProps<T>>::insert(
                    bank_id,
                    proposal_id,
                    new_member_proposal,
                );
                Ok(new_vote_id)
            }
            _ => {
                Err(Error::<T>::CannotTriggerVoteFromCurrentMemberProposalState
                    .into())
            }
        }
    }
    fn poll_membership_proposal(
        prop: MemberProp<T>,
    ) -> Result<Self::PropState, DispatchError> {
        let bank = <BankStores<T>>::get(prop.bank_id())
            .ok_or(Error::<T>::CannotPollProposalIfBaseBankDNE)?;
        let _ = <MemberProps<T>>::get(prop.bank_id(), prop.prop_id())
            .ok_or(Error::<T>::CannotPollProposalIfProposalDNE)?;
        match prop.state() {
            ProposalState::Voting(vote_id) => {
                let vote_outcome =
                    <vote::Module<T>>::get_vote_outcome(vote_id)?;
                if vote_outcome == VoteOutcome::Approved {
                    // approved so try to execute and if not, still approve
                    let new_member_proposal = if let Ok(()) =
                        Self::execute_member_proposal(
                            bank,
                            prop.applicant(),
                            prop.tribute(),
                            prop.shares_requested(),
                        ) {
                        prop.set_state(ProposalState::ApprovedAndExecuted)
                    } else {
                        prop.set_state(ProposalState::ApprovedButNotExecuted)
                    };
                    let ret_state = new_member_proposal.state();
                    <MemberProps<T>>::insert(
                        prop.bank_id(),
                        prop.prop_id(),
                        new_member_proposal,
                    );
                    Ok(ret_state)
                } else {
                    Ok(prop.state())
                }
            }
            _ => Ok(prop.state()),
        }
    }
    fn _burn_shares(
        caller: T::AccountId,
        bank_id: T::BankId,
    ) -> DispatchResult {
        let bank = <BankStores<T>>::get(bank_id)
            .ok_or(Error::<T>::CannotBurnSharesIfBaseBankDNE)?;
        let shares_burned =
            <org::Module<T>>::burn(bank.org(), caller.clone(), None, false)?;
        Self::deposit_event(RawEvent::SharesBurned(
            bank.org(),
            shares_burned.total(),
        ));
        let bank_account_id = Self::bank_account_id(bank_id);
        let balance_in_bank =
            <T as Trait>::Currency::total_balance(&bank_account_id);
        let amt_due = shares_burned.portion().mul_floor(balance_in_bank);
        <T as Trait>::Currency::transfer(
            &bank_account_id,
            &caller,
            amt_due,
            ExistenceRequirement::KeepAlive,
        )?;
        let amt_left = <T as Trait>::Currency::total_balance(&bank_account_id);
        Self::deposit_event(RawEvent::WithdrawnPortion(
            bank_id, amt_due, amt_left,
        ));
        Ok(())
    }
}
