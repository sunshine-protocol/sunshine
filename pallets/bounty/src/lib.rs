#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! Minimal bounty module

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
    DispatchResult,
    ModuleId,
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};
use util::bounty::{
    BountyInformation,
    BountySubmission,
    SubmissionState,
};

// type aliases
type BalanceOf<T> = <<T as Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;
type Bounty<T> = BountyInformation<
    <T as Trait>::BountyId,
    <T as Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
>;
type BountySub<T> = BountySubmission<
    <T as Trait>::BountyId,
    <T as Trait>::SubmissionId,
    <T as Trait>::IpfsReference,
    <T as frame_system::Trait>::AccountId,
    BalanceOf<T>,
    SubmissionState,
>;

pub trait Trait: frame_system::Trait {
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Cid type
    type IpfsReference: Parameter + Member + Default;

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

    /// The foundational foundation
    type Foundation: Get<ModuleId>;

    /// Minimum deposit to post bounty
    type MinDeposit: Get<BalanceOf<Self>>;

    /// Minimum contribution to posted bounty
    type MinContribution: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as Trait>::IpfsReference,
        <T as Trait>::BountyId,
        <T as Trait>::SubmissionId,
        Balance = BalanceOf<T>,
    {
        /// Poster, Initial Amount, Identifier, Bounty Metadata (i.e. github issue reference)
        BountyPosted(AccountId, Balance, BountyId, IpfsReference),
        /// Contributor, This Contribution Amount, Identifier, Full Amount After Contribution, Bounty Metadata
        BountyRaiseContribution(AccountId, Balance, BountyId, Balance, IpfsReference),
        /// Submitter, Bounty Identifier, Amount Requested, Submission Identifier, Bounty Metadata, Submission Metadata
        BountySubmissionPosted(AccountId, BountyId, Balance, SubmissionId, IpfsReference, IpfsReference),
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
        ) -> DispatchResult {
            let depositer = ensure_signed(origin)?;
            ensure!(amount >= T::MinDeposit::get(), Error::<T>::BountyPostMustExceedMinDeposit);
            let imb = T::Currency::withdraw(
                &depositer,
                amount,
                WithdrawReasons::from(WithdrawReason::Transfer),
                ExistenceRequirement::AllowDeath,
            )?;
            let id = Self::bounty_generate_uid();
            let bounty = Bounty::<T>::new(id, info.clone(), depositer.clone(), amount);
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
            let new_bounty = bounty.add_total(amount);
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
            ensure!(submitter != bounty.depositer(), Error::<T>::DepositerCannotSubmitForBounty);
            ensure!(amount <= bounty.total(), Error::<T>::BountySubmissionExceedsTotalAvailableFunding);
            let id = Self::submission_generate_uid();
            let submission = BountySub::<T>::new(bounty_id, id, submission_ref.clone(), submitter.clone(), amount);
            <Submissions<T>>::insert(id, submission);
            Self::deposit_event(RawEvent::BountySubmissionPosted(submitter, bounty_id, amount, id, bounty.info(), submission_ref));
            Ok(())
        }
        #[weight = 0]
        fn approve_bounty_submission(
            origin,
            submission_id: T::SubmissionId,
        ) -> DispatchResult {
            let approver = ensure_signed(origin)?;
            let submission = <Submissions<T>>::get(submission_id).ok_or(Error::<T>::SubmissionDNE)?;
            ensure!(submission.state().awaiting_review(), Error::<T>::SubmissionNotInValidStateToApprove);
            let bounty_id = submission.bounty_id();
            let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
            ensure!(bounty.total() >= submission.amount(), Error::<T>::CannotApproveSubmissionIfAmountExceedsTotalAvailable);
            ensure!(bounty.depositer() == approver, Error::<T>::NotAuthorizedToApproveBountySubmissions);
            // execute payment
            T::Currency::transfer(
                &Self::bounty_account_id(bounty_id),
                &submission.submitter(),
                submission.amount(),
                ExistenceRequirement::KeepAlive,
            )?;
            let new_bounty = bounty.subtract_total(submission.amount());
            let (bounty_info, new_total) = (new_bounty.info(), new_bounty.total());
            // submission approved and executed => can be removed
            <Submissions<T>>::remove(submission_id);
            <Bounties<T>>::insert(bounty_id, new_bounty);
            Self::deposit_event(RawEvent::BountyPaymentExecuted(bounty_id, new_total, submission_id, submission.amount(), submission.submitter(), bounty_info, submission.submission()));
            Ok(())
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
    fn _recursive_remove_bounty(id: T::BountyId) {
        <Bounties<T>>::remove(id);
        <Submissions<T>>::iter()
            .filter(|(_, app)| app.bounty_id() == id)
            .for_each(|(app_id, _)| <Submissions<T>>::remove(app_id));
    }
}
