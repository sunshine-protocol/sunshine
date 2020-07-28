#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! The bounty module allows any `AccountId` to post bounties with rules for approval

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
use frame_system::ensure_signed;
use sp_runtime::{
    traits::{
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
    bounty2::{
        BountyInformation,
        BountySubmission,
        PercentageThreshold,
        ResolutionMetadata,
        SubmissionState,
    },
    organization::OrgRep,
    traits::{
        bounty2::{
            PostBounty,
            SubmitForBounty,
        },
        GroupMembership,
        IDIsAvailable,
        OpenThresholdVote,
    },
};

/// The balances type for this module
type BalanceOf<T> = <<T as donate::Trait>::Currency as Currency<
    <T as frame_system::Trait>::AccountId,
>>::Balance;

pub trait Trait:
    frame_system::Trait + org::Trait + vote::Trait + donate::Trait
{
    /// The overarching event type
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

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

    /// Unambiguous lower bound for bounties posted
    type BountyLowerBound: Get<BalanceOf<Self>>;

    /// Challenge period for bounties
    type ChallengePeriod: Get<Self::BlockNumber>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        <T as frame_system::Trait>::BlockNumber,
        <T as org::Trait>::OrgId,
        <T as org::Trait>::IpfsReference,
        <T as vote::Trait>::VoteId,
        <T as Trait>::BountyId,
        <T as Trait>::SubmissionId,
        Balance = BalanceOf<T>,
    {
        BountyPosted(AccountId, Balance, Option<AccountId>, OrgRep<OrgId>, BountyId, IpfsReference),
        BountySubmissionPosted(AccountId, Option<OrgRep<OrgId>>, BountyId, Balance, SubmissionId, IpfsReference),
        BountySubmissionApprovedAndScheduled(AccountId, BountyId, SubmissionId, AccountId, Balance, AccountId, Option<OrgRep<OrgId>>, BlockNumber, IpfsReference),
        BountySubmissionApprovalChallenged(AccountId, BountyId, SubmissionId, OrgRep<OrgId>, VoteId, IpfsReference),
        BountyPaymentExecuted(BountyId, SubmissionId, AccountId, Balance, AccountId, IpfsReference),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // Bounty Does Not Exist
        BountyDNE,
        DisputeResolvingOrgMustExistToPostBounty,
        SubmissionDNE,
        SubmissionRequestExceedsBounty,
        SubmissionNotInValidStateToApprove,
        NotAuthorizedToApproveBountySubmissions,
        CannotChallengeAfterChallengePeriodEnds,
        SubmissionNotInValidStateForChallenge,
        NotAuthorizedToChallengeApproval,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bounty2 {
        /// Uid generation helper for BountyId
        BountyNonce get(fn bounty_nonce): T::BountyId;

        /// Uid generation helpers for SubmissionId
        SubmissionNonce get(fn submission_nonce): T::SubmissionId;

        // Posted Bounties
        pub Bounties get(fn bounties): map
            hasher(blake2_128_concat) T::BountyId => Option<
                BountyInformation<
                    T::IpfsReference,
                    T::AccountId,
                    BalanceOf<T>,
                    ResolutionMetadata<
                        T::AccountId,
                        OrgRep<T::OrgId>,
                        PercentageThreshold<sp_runtime::Permill>,
                    >,
                >
            >;

        /// Posted Submissions
        pub Submissions get(fn submissions): map
            hasher(blake2_128_concat) T::SubmissionId => Option<
                BountySubmission<
                    T::BountyId,
                    T::AccountId,
                    OrgRep<T::OrgId>,
                    T::IpfsReference,
                    BalanceOf<T>,
                    SubmissionState<
                        T::BlockNumber,
                        T::VoteId,
                    >,
                >
            >;
        /// Frequency with which submissions are polled and dealt with
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
            funding: BalanceOf<T>,
            permissions: ResolutionMetadata<
                T::AccountId,
                OrgRep<T::OrgId>,
                PercentageThreshold<sp_runtime::Permill>,
            >,
        ) -> DispatchResult {
            let poster = ensure_signed(origin)?;
            let (sudo, org) = (permissions.sudo(), permissions.org());
            let id = Self::post_bounty2(poster.clone(), info.clone(), funding, permissions)?;
            Self::deposit_event(RawEvent::BountyPosted(poster, funding, sudo, org, id, info));
            Ok(())
        }
        #[weight = 0]
        fn submit_for_bounty(
            origin,
            bounty_id: T::BountyId,
            team: Option<OrgRep<T::OrgId>>,
            submission_ref: T::IpfsReference,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let submitter = ensure_signed(origin)?;
            let id = Self::submit_for_bounty2(submitter.clone(), bounty_id, team, submission_ref.clone(), amount)?;
            Self::deposit_event(RawEvent::BountySubmissionPosted(submitter, team, bounty_id, amount, id, submission_ref));
            Ok(())
        }
        #[weight = 0]
        fn approve_bounty_submission(
            origin,
            submission_id: T::SubmissionId,
        ) -> DispatchResult {
            let approver = ensure_signed(origin)?;
            let submission = <Submissions<T>>::get(submission_id).ok_or(Error::<T>::SubmissionDNE)?;
            ensure!(submission.awaiting_review(), Error::<T>::SubmissionNotInValidStateToApprove);
            let bounty_id = submission.bounty();
            let bounty = <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
            let permissions = bounty.permissions();
            let authorization = if let Some(s) = permissions.sudo() {
                approver == s
            } else {
                <org::Module<T>>::is_member_of_group(permissions.org().org(), &approver)
            };
            ensure!(authorization, Error::<T>::NotAuthorizedToApproveBountySubmissions);
            let scheduled_time = <frame_system::Module<T>>::block_number() + T::ChallengePeriod::get();
            let approved_submission = submission.set_state(SubmissionState::ApprovedAndScheduled(scheduled_time));
            <Submissions<T>>::insert(submission_id, approved_submission);
            Self::deposit_event(RawEvent::BountySubmissionApprovedAndScheduled(approver, submission.bounty(), submission_id, bounty.poster(), submission.amount(), submission.submitter(), submission.org(), scheduled_time, submission.submission()));
            Ok(())
        }
        #[weight = 0]
        fn challenge_bounty_approval(
            origin,
            submission_id: T::SubmissionId,
        ) -> DispatchResult {
            let challenger = ensure_signed(origin)?;
            let submission = <Submissions<T>>::get(submission_id).ok_or(Error::<T>::SubmissionDNE)?;
            if let Some(exp_block) = submission.approved_and_scheduled() {
                ensure!(exp_block <= <frame_system::Module<T>>::block_number(), Error::<T>::CannotChallengeAfterChallengePeriodEnds);
            } else {
                return Err(Error::<T>::SubmissionNotInValidStateForChallenge.into());
            }
            let bounty = <Bounties<T>>::get(submission.bounty()).ok_or(Error::<T>::BountyDNE)?;
            let permissions = bounty.permissions();
            let authorization = <org::Module<T>>::is_member_of_group(permissions.org().org(), &challenger);
            ensure!(authorization, Error::<T>::NotAuthorizedToChallengeApproval);
            let new_vote_id = <vote::Module<T>>::open_threshold_vote(Some(submission.submission()), permissions.org(), permissions.threshold().pct_to_pass(), permissions.threshold().pct_to_fail(), None)?;
            let challenged_submission = submission.set_state(SubmissionState::ChallengedAndUnderReview(new_vote_id));
            <Submissions<T>>::insert(submission_id, challenged_submission);
            Self::deposit_event(RawEvent::BountySubmissionApprovalChallenged(challenger, submission.bounty(), submission_id, permissions.org(), new_vote_id, submission.submission()));
            Ok(())
        }
        fn on_finalize(n: T::BlockNumber) {
            let now = <frame_system::Module<T>>::block_number();
            if now % Self::submission_poll_frequency() == Zero::zero() {
                // go through the submissions
                let sub_iter = <Submissions<T>>::iter();
                sub_iter.filter(|(_, sub)| {
                    if let Some(execute) = sub.approved_and_scheduled() {
                        execute >= now
                    } else {
                        false
                    }
                }).for_each(|(sid, s)| {
                    let sub: BountySubmission<_,_,_,_,_,_> = s;
                    if let Some(bounty) = <Bounties<T>>::get(sub.bounty()) {
                        let expected_amt = sub.amount();
                        if let Some(paid_bounty) = bounty.pay_out_funding(expected_amt) {
                            // TODO: if let Some make bounty transfer
                            if let Some(remainder) = Self::make_bounty_transfer(
                                &bounty.poster(),
                                &sub.submitter(),
                                sub.org(),
                                expected_amt
                            ) {
                                let paid_amount = expected_amt - remainder;
                                let updated_bounty = bounty.pay_out_funding(paid_amount)
                                    .expect("expected_amt > paid_amount => pay out funding will succeed with less, QED");
                                let updated_submission = sub.pay_out_amount(paid_amount)
                                    .expect("paid_amount > expected_amt = sub.amount(), QED");
                                <Bounties<T>>::insert(sub.bounty(), updated_bounty);
                                <Submissions<T>>::insert(sid, updated_submission);
                            } else {
                                // update storage items to reflect paid out submission
                                if paid_bounty.funding_reserved() == Zero::zero() {
                                    <Bounties<T>>::remove(sub.bounty());
                                    // TODO: remove all submissions with this bounty identifier
                                } else {
                                    <Bounties<T>>::insert(sub.bounty(), paid_bounty);
                                }
                                <Submissions<T>>::remove(sid);
                            }
                        }
                    }
                });
                // TODO
                // if approved && past challenge period, execute payment
                // if challenged, poll and get response
                // if rejected, remove the submission
            }
        }
    }
}

impl<T: Trait> Module<T> {
    fn bounty_id_is_available(id: T::BountyId) -> bool {
        <Bounties<T>>::get(id).is_none()
    }
    fn bounty_generate_unique_id() -> T::BountyId {
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
    fn submission_generate_unique_id() -> T::SubmissionId {
        let mut id_counter = <SubmissionNonce<T>>::get() + 1u32.into();
        while !Self::submission_id_is_available(id_counter) {
            id_counter += 1u32.into();
        }
        <SubmissionNonce<T>>::put(id_counter);
        id_counter
    }
    fn make_bounty_transfer(
        poster: &T::AccountId,
        submitter: &T::AccountId,
        recipient: Option<OrgRep<T::OrgId>>,
        amount: BalanceOf<T>,
    ) -> Option<BalanceOf<T>> {
        if let Some(o) = recipient {
            if let Ok(remainder) =
                <donate::Module<T>>::donate(poster, o, amount)
            {
                if T::Currency::transfer(
                    poster,
                    submitter,
                    remainder,
                    ExistenceRequirement::KeepAlive,
                )
                .is_ok()
                {
                    None
                } else {
                    Some(amount - remainder)
                }
            } else {
                // if donate fails, just try to transfer to submitter
                if T::Currency::transfer(
                    poster,
                    submitter,
                    amount,
                    ExistenceRequirement::KeepAlive,
                )
                .is_ok()
                {
                    None
                } else {
                    Some(amount)
                }
            }
        } else if T::Currency::transfer(
            poster,
            submitter,
            amount,
            ExistenceRequirement::KeepAlive,
        )
        .is_ok()
        {
            None
        } else {
            Some(amount)
        }
    }
}

impl<T: Trait>
    PostBounty<
        T::AccountId,
        T::IpfsReference,
        BalanceOf<T>,
        ResolutionMetadata<
            T::AccountId,
            OrgRep<T::OrgId>,
            PercentageThreshold<sp_runtime::Permill>,
        >,
    > for Module<T>
{
    type BountyId = T::BountyId;
    fn post_bounty2(
        poster: T::AccountId,
        info: T::IpfsReference,
        funding: BalanceOf<T>,
        permissions: ResolutionMetadata<
            T::AccountId,
            OrgRep<T::OrgId>,
            PercentageThreshold<sp_runtime::Permill>,
        >,
    ) -> Result<Self::BountyId, DispatchError> {
        ensure!(
            !<org::Module<T>>::id_is_available(permissions.org().org()),
            Error::<T>::DisputeResolvingOrgMustExistToPostBounty
        );
        T::Currency::reserve(&poster, funding)?;
        let bounty = BountyInformation::new(info, poster, funding, permissions);
        let id: T::BountyId = Self::bounty_generate_unique_id();
        <Bounties<T>>::insert(id, bounty);
        Ok(id)
    }
}

impl<T: Trait>
    SubmitForBounty<
        T::AccountId,
        T::BountyId,
        OrgRep<T::OrgId>,
        T::IpfsReference,
        BalanceOf<T>,
    > for Module<T>
{
    type SubmissionId = T::SubmissionId;
    fn submit_for_bounty2(
        submitter: T::AccountId,
        bounty_id: T::BountyId,
        team: Option<OrgRep<T::OrgId>>,
        submission_ref: T::IpfsReference,
        amount: BalanceOf<T>,
    ) -> Result<Self::SubmissionId, DispatchError> {
        let bounty =
            <Bounties<T>>::get(bounty_id).ok_or(Error::<T>::BountyDNE)?;
        ensure!(
            amount <= bounty.funding_reserved(),
            Error::<T>::SubmissionRequestExceedsBounty
        );
        let submission = BountySubmission::new(
            bounty_id,
            submitter,
            team,
            submission_ref,
            amount,
        );
        let id: T::SubmissionId = Self::submission_generate_unique_id();
        <Submissions<T>>::insert(id, submission);
        Ok(id)
    }
}
