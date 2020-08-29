#![recursion_limit = "256"]
//! # RFC Module
//! This module expresses a governance process for improvement proposals
//!
//! - [`rfc::Trait`](./trait.Trait.html)
//! - [`Call`](./enum.Call.html)
//!
//! ## Overview
//!
//! This pallet allows orgs to govern sets of doc references
//! relevant to org governance.
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
    storage::IterableStorageDoubleMap,
    traits::Get,
    Parameter,
};
use frame_system::{
    ensure_signed,
    Trait as System,
};
use org::Trait as Org;
use sp_runtime::{
    traits::{
        AtLeast32Bit,
        AtLeast32BitUnsigned,
        MaybeSerializeDeserialize,
        Member,
        Zero,
    },
    DispatchError,
    DispatchResult,
    Permill,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::{
    organization::OrgRep,
    rfp::{
        DocProposal,
        DocRef,
        DocState,
        RfcBoard,
    },
    traits::{
        ConfigureThreshold,
        DocGovernance,
        GetVoteOutcome,
        GroupMembership,
    },
    vote::{
        ThresholdInput,
        VoteOutcome,
        XorThreshold,
    },
};
use vote::Trait as Vote;

// type aliases
type Gov<T> = RfcBoard<
    <T as Trait>::CommitteeId,
    <T as org::Trait>::OrgId,
    <T as System>::AccountId,
    <T as vote::Trait>::ThresholdId,
    DocRef<<T as Trait>::DocId, <T as Org>::Cid>,
>;
type Threshold<T> = ThresholdInput<
    OrgRep<<T as org::Trait>::OrgId>,
    XorThreshold<<T as vote::Trait>::Signal, Permill>,
>;
type Proposal<T> = DocProposal<
    <T as Trait>::CommitteeId,
    <T as Trait>::ProposalId,
    <T as Org>::Cid,
    <T as System>::AccountId,
    DocState<<T as vote::Trait>::VoteId, <T as Trait>::DocId>,
>;

pub trait Trait: System + Org + Vote {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as System>::Event>;

    /// The committee identifier
    type CommitteeId: Parameter
        + Member
        + AtLeast32BitUnsigned
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + PartialOrd
        + PartialEq
        + Zero;

    /// Identifier for proposals
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

    /// Identifier for docs approved by governance
    type DocId: Parameter
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

    /// Max number of committees for one org
    type MaxCommitteePerOrg: Get<u32>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as System>::AccountId,
        <T as Org>::Cid,
        <T as Vote>::VoteId,
        <T as Trait>::CommitteeId,
        <T as Trait>::ProposalId,
        <T as Trait>::DocId,
    {
        RfProcessOpened(AccountId, CommitteeId),
        DocProposed(AccountId, CommitteeId, ProposalId, Cid),
        VoteTriggered(AccountId, CommitteeId, ProposalId, Cid, VoteId),
        SudoApproved(AccountId, CommitteeId, ProposalId, DocId, Cid),
        ProposalPolled(CommitteeId, ProposalId, DocState<VoteId, DocId>),
        RfProcessClosed(AccountId, CommitteeId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Committee Does Not Exist
        CommitteeDNE,
        ProposalDNE,
        NotAuthorizedForAccount,
        CommitteeCountExceedsLimitPerOrg,
        ThresholdCannotBeSetForOrg,
        CannotTriggerVoteFromCurrentProposalState,
        DocInsertionFailed,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Rfp {
        /// The nonce for unique committee id generation
        CommitteeIdCounter get(fn committee_id_counter): T::CommitteeId;

        /// Counter for generating unique proposal identifiers
        ProposalNonceMap get(fn proposal_nonce_map): map
            hasher(blake2_128_concat) T::CommitteeId => T::ProposalId;

        /// Total number of committees registered in this module
        pub TotalCommitteeCount get(fn total_committee_count): u32;

        /// The total number of committee accounts per org
        pub OrgCommitteeCount get(fn org_committee_count): map
            hasher(blake2_128_concat) T::OrgId => u32;

        /// The store for committee governance
        /// -> keyset acts as canonical set for unique CommitteeIds
        pub Committees get(fn committees): map
            hasher(blake2_128_concat) T::CommitteeId => Option<Gov<T>>;

        /// Proposals to improve things in the committees jurisdiction
        pub Proposals get(fn proposals): double_map
            hasher(blake2_128_concat) T::CommitteeId,
            hasher(blake2_128_concat) T::ProposalId => Option<Proposal<T>>;
        /// Frequency for which all proposals are polled and pushed along
        ProposalPollFrequency get(fn proposal_poll_frequency) config(): T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        pub fn open(
            origin,
            org: T::OrgId,
            controller: Option<T::AccountId>,
            threshold: Threshold<T>,
        ) -> DispatchResult {
            let opener = ensure_signed(origin)?;
            ensure!(
                <org::Module<T>>::is_member_of_group(org, &opener),
                Error::<T>::NotAuthorizedForAccount
            );
            let new_count = <OrgCommitteeCount<T>>::get(org) + 1;
            ensure!(
                new_count <= T::MaxCommitteePerOrg::get(),
                Error::<T>::CommitteeCountExceedsLimitPerOrg
            );
            ensure!(
                threshold.org().org() == org,
                Error::<T>::ThresholdCannotBeSetForOrg
            );
            // register input threshold
            let threshold_id = <vote::Module<T>>::register_threshold(threshold)?;
            // generate new committee identifier
            let id = Self::generate_committee_uid();
            // create new committee governance object
            let committee = Gov::<T>::new(id, org, controller, threshold_id);
            <Committees<T>>::insert(id, committee);
            // put new org committee count
            <OrgCommitteeCount<T>>::insert(org, new_count);
            // iterate total committee count
            <TotalCommitteeCount>::mutate(|count| *count += 1u32);
            Self::deposit_event(RawEvent::RfProcessOpened(opener, id));
            Ok(())
        }
        #[weight = 0]
        pub fn propose_doc(
            origin,
            committee_id: T::CommitteeId,
            doc_ref: T::Cid,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let id = Self::_propose_doc(caller.clone(), committee_id, doc_ref.clone())?;
            Self::deposit_event(RawEvent::DocProposed(caller, committee_id, id, doc_ref));
            Ok(())
        }
        #[weight = 0]
        pub fn trigger_vote(
            origin,
            committee_id: T::CommitteeId,
            proposal_id: T::ProposalId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let (cid, vid) = Self::_trigger_vote_on_proposal(&caller, committee_id, proposal_id)?;
            Self::deposit_event(RawEvent::VoteTriggered(caller, committee_id, proposal_id, cid, vid));
            Ok(())
        }
        #[weight = 0]
        pub fn sudo_approve(
            origin,
            committee_id: T::CommitteeId,
            proposal_id: T::ProposalId,
        ) -> DispatchResult {
            let sudo = ensure_signed(origin)?;
            let (doc_index, cid) = Self::_sudo_approve_proposal(&sudo, committee_id, proposal_id)?;
            Self::deposit_event(RawEvent::SudoApproved(sudo, committee_id, proposal_id, doc_index, cid));
            Ok(())
        }
        #[weight = 0]
        pub fn close(
            origin,
            committee_id: T::CommitteeId,
        ) -> DispatchResult {
            let closer = ensure_signed(origin)?;
            let committee = <Committees<T>>::get(committee_id).ok_or(Error::<T>::CommitteeDNE)?;
            ensure!(
                committee.is_controller(&closer),
                Error::<T>::NotAuthorizedForAccount
            );
            <Committees<T>>::remove(committee_id);
            <OrgCommitteeCount<T>>::mutate(committee.org(), |count| *count -= 1);
            <TotalCommitteeCount>::mutate(|c| *c -= 1);
            Self::deposit_event(RawEvent::RfProcessClosed(closer, committee_id));
            Ok(())
        }
        fn on_finalize(_n: T::BlockNumber) {
            if <frame_system::Module<T>>::block_number() % Self::proposal_poll_frequency() == Zero::zero() {
                <Proposals<T>>::iter().for_each(|(_, _, prop)| {
                    let (committee_id, proposal_id) = (prop.committee_id(), prop.proposal_id());
                    if let Ok(state) = Self::poll_proposal(prop) {
                        Self::deposit_event(RawEvent::ProposalPolled(committee_id, proposal_id, state));
                    }
                });
            }
        }
    }
}

impl<T: Trait> Module<T> {
    pub fn generate_committee_uid() -> T::CommitteeId {
        let mut committee_counter =
            <CommitteeIdCounter<T>>::get() + 1u32.into();
        while <Committees<T>>::get(committee_counter).is_some() {
            committee_counter += 1u32.into();
        }
        <CommitteeIdCounter<T>>::put(committee_counter);
        committee_counter
    }
    pub fn generate_proposal_uid(
        committee_id: T::CommitteeId,
    ) -> T::ProposalId {
        let mut proposal_counter =
            <ProposalNonceMap<T>>::get(committee_id) + 1u32.into();
        while <Proposals<T>>::get(committee_id, proposal_counter).is_some() {
            proposal_counter += 1u32.into();
        }
        <ProposalNonceMap<T>>::insert(committee_id, proposal_counter);
        proposal_counter
    }
}

impl<T: Trait> DocGovernance<T::CommitteeId, T::Cid, T::AccountId, Proposal<T>>
    for Module<T>
{
    type ProposalId = T::ProposalId;
    type VoteId = T::VoteId;
    type PropState = DocState<T::VoteId, T::DocId>;
    type DocIndex = T::DocId;
    fn _propose_doc(
        caller: T::AccountId,
        committee_id: T::CommitteeId,
        doc_ref: T::Cid,
    ) -> Result<Self::ProposalId, DispatchError> {
        let committee = <Committees<T>>::get(committee_id)
            .ok_or(Error::<T>::CommitteeDNE)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(committee.org(), &caller),
            Error::<T>::NotAuthorizedForAccount
        );
        let id = Self::generate_proposal_uid(committee_id);
        let proposal = Proposal::<T>::new(committee_id, id, doc_ref, caller);
        <Proposals<T>>::insert(committee_id, id, proposal);
        Ok(id)
    }
    fn _trigger_vote_on_proposal(
        caller: &T::AccountId,
        committee_id: T::CommitteeId,
        proposal_id: Self::ProposalId,
    ) -> Result<(T::Cid, Self::VoteId), DispatchError> {
        let committee = <Committees<T>>::get(committee_id)
            .ok_or(Error::<T>::CommitteeDNE)?;
        ensure!(
            <org::Module<T>>::is_member_of_group(committee.org(), &caller),
            Error::<T>::NotAuthorizedForAccount
        );
        let proposal = <Proposals<T>>::get(committee_id, proposal_id)
            .ok_or(Error::<T>::ProposalDNE)?;
        match proposal.state() {
            DocState::WaitingForApproval => {
                // dispatch vote with bank's default threshold
                let new_vote_id = <vote::Module<T>>::invoke_threshold(
                    committee.threshold_id(),
                    None, // TODO: use vote info ref here instead of None
                    None,
                )?;
                let cid = proposal.doc();
                let new_proposal =
                    proposal.set_state(DocState::Voting(new_vote_id));
                <Proposals<T>>::insert(committee_id, proposal_id, new_proposal);
                Ok((cid, new_vote_id))
            }
            _ => {
                Err(Error::<T>::CannotTriggerVoteFromCurrentProposalState
                    .into())
            }
        }
    }
    fn _sudo_approve_proposal(
        caller: &T::AccountId,
        committee_id: T::CommitteeId,
        proposal_id: Self::ProposalId,
    ) -> Result<(T::DocId, T::Cid), DispatchError> {
        let committee = <Committees<T>>::get(committee_id)
            .ok_or(Error::<T>::CommitteeDNE)?;
        ensure!(
            committee.is_controller(caller),
            Error::<T>::NotAuthorizedForAccount
        );
        let proposal = <Proposals<T>>::get(committee_id, proposal_id)
            .ok_or(Error::<T>::ProposalDNE)?;
        match proposal.state() {
            DocState::WaitingForApproval => {
                let cid = proposal.doc();
                let (doc_id, new_committee) = committee
                    .add_doc(cid.clone())
                    .ok_or(Error::<T>::DocInsertionFailed)?;
                let new_proposal =
                    proposal.set_state(DocState::ApprovedAndAdded(doc_id));
                <Proposals<T>>::insert(committee_id, proposal_id, new_proposal);
                <Committees<T>>::insert(committee_id, new_committee);
                Ok((doc_id, cid))
            }
            _ => {
                Err(Error::<T>::CannotTriggerVoteFromCurrentProposalState
                    .into())
            }
        }
    }
    fn poll_proposal(
        prop: Proposal<T>,
    ) -> Result<Self::PropState, DispatchError> {
        let committee = <Committees<T>>::get(prop.committee_id())
            .ok_or(Error::<T>::CommitteeDNE)?;
        let _ = <Proposals<T>>::get(prop.committee_id(), prop.proposal_id())
            .ok_or(Error::<T>::ProposalDNE)?;
        match prop.state() {
            DocState::Voting(vote_id) => {
                let vote_outcome =
                    <vote::Module<T>>::get_vote_outcome(vote_id)?;
                // TODO: handle when approved but DocInsertionFailed is thrown (i.e. when existing cid is added to the set)
                // -> could periodically purge proposals based on checks of existence in the blockchain
                if vote_outcome == VoteOutcome::Approved {
                    let cid = prop.doc();
                    let (doc_id, new_committee) = committee
                        .add_doc(cid)
                        .ok_or(Error::<T>::DocInsertionFailed)?;
                    let new_prop =
                        prop.set_state(DocState::ApprovedAndAdded(doc_id));
                    <Proposals<T>>::insert(
                        prop.committee_id(),
                        prop.proposal_id(),
                        new_prop,
                    );
                    <Committees<T>>::insert(prop.committee_id(), new_committee);
                    Ok(DocState::ApprovedAndAdded(doc_id))
                } else {
                    Ok(prop.state())
                }
            }
            _ => Ok(prop.state()),
        }
    }
}
