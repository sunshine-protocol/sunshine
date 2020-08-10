#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Bounty pallet with refundable contributions and more contributor voting rights

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
        WithdrawReason,
        WithdrawReasons,
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
    bounty::{
        BountyInfo2,
        BountyState,
        BountySubmission,
        SubmissionState2,
    },
    grant::ChallengeNorms,
    share::SimpleShareGenesis,
    traits::{
        AccessGenesis,
        GetVoteOutcome,
    },
    vote::VoteOutcome,
};

// type aliases
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type Bounty<T> = BountyInfo2<
    <T as vote::Trait>::IpfsReference,
    ChallengeNorms<<T as frame_system::Trait>::AccountId, Permill>,
    BalanceOf<T>,
    BountyState<<T as vote::Trait>::VoteId>,
>;
type BountySub<T> = BountySubmission<
    <T as Trait>::BountyId,
    <T as vote::Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    SubmissionState2<
        <T as frame_system::Trait>::BlockNumber,
        <T as vote::Trait>::VoteId,
    >,
>;

pub trait Trait: frame_system::Trait + vote::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// The currency type
    type Currency: Currency<Self::AccountId>
        + ReservableCurrency<Self::AccountId>;

    /// The bounty post identifier
    type BountyId: Parameter
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

    /// The bounty submission identifier
    type SubmissionId: Parameter
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

    /// Period for which every spend decision can be challenged with a veto and/or refund request
    type ChallengePeriod: Get<Self::BlockNumber>;

    /// The foundational foundation
    type Foundation: Get<ModuleId>;

    /// Minimum deposit to post bounty
    type MinDeposit: Get<BalanceOf<Self>>;

    /// Minimum contribution to posted bounty
    type MinContribution: Get<BalanceOf<Self>>;

    /// Minimum veto threshold
    type MinVetoThreshold: Get<Permill>;

    /// Minimum refund threshold
    type MinRefundThreshold: Get<Permill>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as vote::Trait>::IpfsReference,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BountyId,
        <T as Trait>::SubmissionId,
        Balance = BalanceOf<T>,
    {
        /// Poster, Initial Amount, Identifier, Bounty Metadata (i.e. github issue reference)
        BountyPosted(AccountId, Balance, BountyId, IpfsReference),
        /// Contributor, This Contribution Amount, Identifier, Full Amount After Contribution, Bounty Metadata
        BountyRaiseContribution(AccountId, Balance, BountyId, Balance, IpfsReference),
        /// Contributors challenged and achieved threshold support to execute a refund for all contributions
        /// -> bounty identifier, amt to contributors, amt to depositer (as de facto remainder_recipient)
        BountyRefunded(BountyId, Balance, Balance),
        /// Submitter, Bounty Identifier, Amount Requested, Submission Identifier, Bounty Metadata, Submission Metadata
        BountySubmissionPosted(AccountId, BountyId, Balance, SubmissionId, IpfsReference, IpfsReference),
        /// Submission Identifier, Bounty Identifier, Requested Amount
        SubmissionApprovedButPaymentFailed(SubmissionId, BountyId, Balance),
        /// Submission Identifier, Bounty Identifier, Requested Amount
        SpendChallengePassedAndSubmissionRejected(SubmissionId, BountyId, Balance),
        /// Vote identifier for Challenge Results, Bounty Identifier, Amount Posted
        BountyRefundChallengeRejected(VoteId, BountyId, Balance),
        /// Bounty Identifier, Full Amount Left After Payment, Submission Identifier, Amount Requested, Bounty Metadata, Submission Metadata
        BountyPaymentExecuted(BountyId, Balance, SubmissionId, Balance, AccountId, IpfsReference, IpfsReference),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Bounty Does Not Exist
        BountyDNE,
        SubmissionDNE,
        BountyPostMustExceedMinDeposit,
        ContributionMustExceedModuleMin,
        DepositerCannotSubmitForBounty,
        BountySubmissionExceedsTotalAvailableFunding,
        SubmissionNotInValidStateToApprove,
        CannotApproveSubmissionIfAmountExceedsTotalAvailable,
        NotAuthorizedToApproveBountySubmissions,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bounty {
        /// Uid generation helper for BountyId
        BountyNonce get(fn bounty_nonce): T::BountyId;

        /// Uid generation helpers for SubmissionId
        SubmissionNonce get(fn submission_nonce): T::SubmissionId;

        /// Posted Bounties
        pub Bounties get(fn bounties): map
            hasher(blake2_128_concat) T::BountyId => Option<Bounty<T>>;
        /// Tips for existing Bounties
        pub BountyTips get(fn bounty_tips): double_map
            hasher(blake2_128_concat) T::BountyId,
            hasher(blake2_128_concat) T::AccountId => Option<BalanceOf<T>>;

        /// Posted Submissions
        pub Submissions get(fn submissions): map
            hasher(blake2_128_concat) T::SubmissionId => Option<BountySub<T>>;

        /// Every this period, poll the status of refund vote challenges and push results
        pub BountyPollFrequency get(fn bounty_poll_frequency) config(): T::BlockNumber;
        /// Every this period, poll contributor veto votes against submission approvals and push results
        pub SubmissionPollFrequency get(fn submission_poll_frequency) config(): T::BlockNumber;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn post_bounty(
            origin,
            info: T::IpfsReference,
            amount: BalanceOf<T>,
            veto_threshold: Permill,
            refund_threshold: Permill,
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;
            ensure!(amount >= T::MinDeposit::get(), Error::<T>::BountyPostMustExceedMinDeposit);
            let imb = T::Currency::withdraw(
                &depositer,
                amount,
                WithdrawReasons::from(WithdrawReason::Transfer),
                ExistenceRequirement::AllowDeath,
            )?;
            let bounty = Bounty::<T>::new(info.clone(), ChallengeNorms::new(depositer.clone(), veto_threshold, refund_threshold), amount);
            let id = Self::bounty_generate_uid();
            T::Currency::resolve_creating(&Self::bounty_account_id(id), imb);
            <Bounties<T>>::insert(id, bounty);
            <BountyTips<T>>::insert(id, &depositer, amount);
            Self::deposit_event(RawEvent::BountyPosted(depositer, amount, id, info));
            Ok(())
        }
        #[weight = 0]
        fn contribute_to_bounty(
            origin,
            bounty_id: T::BountyId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let contributor = ensure_signed(origin)?;
            ensure!(amount >= T::MinContribution::get(), Error::<T>::ContributionMustExceedModuleMin);
            let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
            T::Currency::transfer(
                &contributor,
                &Self::bounty_account_id(bounty_id),
                amount,
                ExistenceRequirement::KeepAlive,
            )?;
            let new_amount = if let Some(a) = <BountyTips<T>>::get(bounty_id, &contributor) {
                amount + a
            } else {
                amount
            };
            let new_bounty = bounty.add_funds(amount);
            let total = new_bounty.total();
            <BountyTips<T>>::insert(bounty_id, &contributor, new_amount);
            <Bounties<T>>::insert(bounty_id, new_bounty);
            Self::deposit_event(RawEvent::BountyRaiseContribution(contributor, amount, bounty_id, total, bounty.info()));
            Ok(())
        }
        #[weight = 0]
        fn submit_for_bounty(
            origin,
            bounty_id: T::BountyId,
            submission_ref: T::IpfsReference,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
            ensure!(submitter != bounty.gov().leader(), Error::<T>::DepositerCannotSubmitForBounty);
            ensure!(amount <= bounty.total(), Error::<T>::BountySubmissionExceedsTotalAvailableFunding);
            let submission = BountySub::<T>::new(bounty_id, submission_ref.clone(), submitter.clone(), amount);
            let id = Self::submission_generate_uid();
            <Submissions<T>>::insert(id, submission);
            Self::deposit_event(RawEvent::BountySubmissionPosted(submitter, bounty_id, amount, id, bounty.info(), submission_ref));
            Ok(())
        }
        #[weight = 0]
        fn approve_bounty_submission(
            origin,
            submission_id: T::SubmissionId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
        #[weight = 0]
        fn reject_bounty_submission(
            origin,
            submission_id: T::SubmissionId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
        #[weight = 0]
        fn trigger_refund_vote(
            origin,
            bounty_id: T::BountyId,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            Ok(())
        }
        fn on_finalize(_n: T::BlockNumber) {
            let now = <frame_system::Module<T>>::block_number();
            // poll submissions and execute approved and scheduled submissions
            if now % Self::submission_poll_frequency() == Zero::zero() {
                for (sub_id, sub) in <Submissions<T>>::iter() {
                    match sub.state() {
                        SubmissionState2::ApprovedAndScheduled(n) => {
                            if now >= n {
                                // TODO: make this path infallible
                                // approve and execute submission payment
                                if !Self::approve_and_execute_payment(sub_id).is_ok() {
                                    Self::deposit_event(RawEvent::SubmissionApprovedButPaymentFailed(sub_id, sub.bounty_id(), sub.amount()));
                                }
                            }
                        }
                        SubmissionState2::ChallengedAndUnderReview(v) => {
                            let status = <vote::Module<T>>::get_vote_outcome(v).expect("dispatched votes are never cleared by default, qed");
                            match status {
                                VoteOutcome::Approved => {
                                    // => the submission is rejected because this vote was a challenge to an approval by the depositer
                                    <Submissions<T>>::remove(sub_id);
                                    Self::deposit_event(RawEvent::SpendChallengePassedAndSubmissionRejected(sub_id, sub.bounty_id(), sub.amount()));
                                },
                                VoteOutcome::Rejected => {
                                    // TODO: make this path infallible
                                    // => the submission is approved because this vote was a challenge to an approval by the depositer
                                    if !Self::approve_and_execute_payment(sub_id).is_ok() {
                                        Self::deposit_event(RawEvent::SubmissionApprovedButPaymentFailed(sub_id, sub.bounty_id(), sub.amount()));
                                    }
                                },
                                _ => (),
                            }
                        }
                        _ => (),
                    }
                }
            }
            // poll bounties and execute refunds for bounties under contributor vote to refund
            if now % Self::bounty_poll_frequency() == Zero::zero() {
                for (bid, bty) in <Bounties<T>>::iter() {
                    match bty.state() {
                        BountyState::ChallengedToClose(v) => {
                            let status = <vote::Module<T>>::get_vote_outcome(v).expect("dispatched votes are never cleared by default, qed");
                            match status {
                                VoteOutcome::Approved => {
                                    // => the refund is executed
                                    if let Ok((amt_to_contributors, amt_to_depositer)) = Self::execute_refund(bid, &bty.gov().leader()) {
                                        Self::deposit_event(RawEvent::BountyRefunded(bid, amt_to_contributors, amt_to_depositer));
                                    }
                                },
                                VoteOutcome::Rejected => {
                                    // => the refund is not executed and the bty state is reset to NoPendingChallenges until next challenge
                                    let new_bty = bty.set_state(BountyState::NoPendingChallenges);
                                    let total = new_bty.total();
                                    <Bounties<T>>::insert(bid, new_bty);
                                    Self::deposit_event(RawEvent::BountyRefundChallengeRejected(v, bid, total));
                                },
                                _ => (),
                            }
                        },
                        _ => (),
                    }
                }
            }
        }
    }
}

// ID helpers
impl<T: Trait> Module<T> {
    pub fn bounty_account_id(index: T::BountyId) -> T::AccountId {
        T::Foundation::get().into_sub_account(index)
    }
    fn bounty_id_is_available(id: T::BountyId) -> bool {
        <Bounties<T>>::get(id).is_none()
    }
    fn bounty_generate_uid() -> T::BountyId {
        let mut id_counter = <BountyNonce<T>>::get() + 1u32.into();
        while !Self::bounty_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <BountyNonce<T>>::put(id_counter);
        id_counter
    }
    fn submission_id_is_available(id: T::SubmissionId) -> bool {
        <Submissions<T>>::get(id).is_none()
    }
    fn submission_generate_uid() -> T::SubmissionId {
        let mut id_counter = <SubmissionNonce<T>>::get() + 1u32.into();
        while !Self::submission_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <SubmissionNonce<T>>::put(id_counter);
        id_counter
    }
}

// Runtime helpers
impl<T: Trait> Module<T> {
    fn approve_and_execute_payment(id: T::SubmissionId) -> DispatchResult {
        let submission =
            <Submissions<T>>::get(id).ok_or(Error::<T>::SubmissionDNE)?;
        ensure!(
            submission.state().awaiting_review(),
            Error::<T>::SubmissionNotInValidStateToApprove
        );
        let bounty_id = submission.bounty_id();
        let bounty =
            <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
        ensure!(
            bounty.total() >= submission.amount(),
            Error::<T>::CannotApproveSubmissionIfAmountExceedsTotalAvailable
        );
        // execute payment
        T::Currency::transfer(
            &Self::bounty_account_id(bounty_id),
            &submission.submitter(),
            submission.amount(),
            ExistenceRequirement::KeepAlive,
        )?;
        let new_bounty = bounty.subtract_funds(submission.amount());
        let (bounty_info, new_total) = (new_bounty.info(), new_bounty.total());
        // submission approved and executed => can be removed
        <Submissions<T>>::remove(id);
        <Bounties<T>>::insert(bounty_id, new_bounty);
        Self::deposit_event(RawEvent::BountyPaymentExecuted(
            bounty_id,
            new_total,
            id,
            submission.amount(),
            submission.submitter(),
            bounty_info,
            submission.submission(),
        ));
        Ok(())
    }
    // remainder recipient should be the depositer, aka bounty.gov().leader()
    fn execute_refund(
        id: T::BountyId,
        remainder_recipient: &T::AccountId,
    ) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
        let from = Self::bounty_account_id(id);
        let mut total = T::Currency::total_balance(&from);
        let contributors: SimpleShareGenesis<T::AccountId, BalanceOf<T>> =
            <BountyTips<T>>::iter()
                .filter(|(i, acc, amt)| i == &id)
                .map(|(_, ac, amt)| (ac, amt))
                .collect::<Vec<(T::AccountId, BalanceOf<T>)>>()
                .into();
        let num_of_accounts: u32 =
            contributors.account_ownership().len() as u32;
        if num_of_accounts == 1 {
            T::Currency::transfer(
                &from,
                &contributors.account_ownership()[0].0,
                total,
                ExistenceRequirement::AllowDeath,
            )?;
            // Self::recursive_remove_bounty(id);
            Ok((total, BalanceOf::<T>::zero()))
        } else {
            todo!()
            // let mut total_to_contributors = BalanceOf::<T>::zero();
            // let den: T::Signal = contributors.total();
            // for (acc, nom) in contributors.account_ownership().iter() {
            //     let due_proportion =
            //         Permill::from_rational_approximation(nom, &den);
            //     let due_amount: BalanceOf<T> = due_proportion * total;
            //     T::Currency::transfer(
            //         &from,
            //         &acc,
            //         due_amount,
            //         ExistenceRequirement::AllowDeath,
            //     )?;
            //     total -= due_amount;
            //     total_to_contributors += due_amount;
            // }
            // // send remainder
            // T::Currency::transfer(
            //     &from,
            //     remainder_recipient,
            //     total,
            //     ExistenceRequirement::AllowDeath,
            // )?;
            // // Self::recursive_remove_bounty(id);
            // Ok((total_to_contributors, total))
        }
    }
    fn recursive_remove_bounty(id: T::BountyId) {
        <Bounties<T>>::remove(id);
        <Submissions<T>>::iter()
            .filter(|(_, app)| app.bounty_id() == id)
            .for_each(|(app_id, _)| <Submissions<T>>::remove(app_id));
    }
}
