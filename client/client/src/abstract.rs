use crate::{
    error::Result,
    srml::{
        bank::*,
        bounty::*,
        donate::*,
        org::*,
        vote::*,
    },
    Client,
};
use async_trait::async_trait;
use codec::Decode;
use libipld::store::Store;
use sp_core::crypto::{
    Pair,
    Ss58Codec,
};
use sp_runtime::{
    traits::{
        IdentifyAccount,
        SignedExtension,
        Verify,
    },
    Permill,
};
use substrate_subxt::{
    sp_core,
    sp_runtime,
    system::System,
    Runtime,
    SignedExtra,
};
use sunshine_bounty_utils::{
    court::ResolutionMetadata,
    vote::VoterView,
};

#[async_trait]
pub trait AbstractClient<
    T: Runtime + Org + Vote + Donate + Bank + Bounty,
    P: Pair,
>: Send + Sync
{
    // org module calls
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>>;
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>>;
    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>>;
    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>>;
    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>>;
    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>>;
    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>>;
    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>>;
    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>>;
    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>>;
    // vote module calls
    async fn create_signal_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>>;
    async fn create_percent_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_threshold: Permill,
        turnout_threshold: Option<Permill>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>>;
    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>>;
    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<<T as Org>::IpfsReference>,
    ) -> Result<VotedEvent<T>>;
    // donate module calls
    async fn make_prop_donation_with_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>>;
    async fn make_prop_donation_without_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>>;
    // bank module calls
    async fn open_org_bank_account(
        &self,
        seed: BalanceOf<T>,
        hosting_org: <T as Org>::OrgId,
        bank_operator: Option<<T as System>::AccountId>,
    ) -> Result<OrgBankAccountOpenedEvent<T>>;
    // bounty module calls
    async fn account_posts_bounty(
        &self,
        description: <T as Org>::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>,
        acceptance_committee: ResolutionMetadata<
            <T as Org>::OrgId,
            <T as Vote>::Signal,
            <T as System>::BlockNumber,
        >,
        supervision_committee: Option<
            ResolutionMetadata<
                <T as Org>::OrgId,
                <T as Vote>::Signal,
                <T as System>::BlockNumber,
            >,
        >,
    ) -> Result<BountyPostedEvent<T>>;
    async fn account_applies_for_bounty(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        description: <T as Org>::IpfsReference,
        total_amount: BalanceOf<T>,
    ) -> Result<BountyApplicationSubmittedEvent<T>>;
    async fn account_triggers_application_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        new_grant_app_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationReviewTriggeredEvent<T>>;
    async fn account_sudo_approves_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<SudoApprovedApplicationEvent<T>>;
    async fn poll_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationPolledEvent<T>>;
    async fn submit_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
        submission_reference: <T as Org>::IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<MilestoneSubmittedEvent<T>>;
    async fn trigger_milestone_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneReviewTriggeredEvent<T>>;
    async fn sudo_approves_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneSudoApprovedEvent<T>>;
    async fn poll_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestonePolledEvent<T>>;
    fn subxt(&self) -> &substrate_subxt::Client<T>;
}

#[async_trait]
impl<T, P, I> AbstractClient<T, P> for Client<T, P, I>
where
    T: Runtime + Org + Vote + Donate + Bank + Bounty,
    <T as System>::AccountId: Into<<T as System>::Address> + Ss58Codec,
    T::Signature: Decode + From<P::Signature>,
    <T::Signature as Verify>::Signer:
        From<P::Public> + IdentifyAccount<AccountId = <T as System>::AccountId>,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    P: Pair,
    <P as Pair>::Public: Into<<T as System>::AccountId>,
    <P as Pair>::Seed: From<[u8; 32]>,
    I: Store + Send + Sync,
{
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>> {
        self.register_flat_org(sudo, parent_org, constitution, members)
            .await
    }

    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>> {
        self.register_weighted_org(
            sudo,
            parent_org,
            constitution,
            weighted_members,
        )
        .await
    }

    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        self.issue_shares(organization, who, shares).await
    }

    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        self.burn_shares(organization, who, shares).await
    }

    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        self.batch_issue_shares(organization, new_accounts).await
    }

    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        self.batch_burn_shares(organization, old_accounts).await
    }

    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>> {
        self.reserve_shares(org, who).await
    }

    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>> {
        self.unreserve_shares(org, who).await
    }

    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>> {
        self.lock_shares(org, who).await
    }

    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>> {
        self.unlock_shares(org, who).await
    }

    async fn create_signal_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        self.create_signal_threshold_vote(
            topic,
            organization,
            support_requirement,
            turnout_requirement,
            duration,
        )
        .await
    }

    async fn create_percent_threshold_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_threshold: Permill,
        turnout_threshold: Option<Permill>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        self.create_percent_threshold_vote(
            topic,
            organization,
            support_threshold,
            turnout_threshold,
            duration,
        )
        .await
    }

    async fn create_unanimous_consent_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        self.create_unanimous_consent_vote(topic, organization, duration)
            .await
    }

    async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<<T as Org>::IpfsReference>,
    ) -> Result<VotedEvent<T>> {
        self.submit_vote(vote_id, direction, justification).await
    }

    async fn make_prop_donation_with_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>> {
        self.make_prop_donation_with_fee(org, amt).await
    }

    async fn make_prop_donation_without_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>> {
        self.make_prop_donation_without_fee(org, amt).await
    }

    async fn open_org_bank_account(
        &self,
        seed: BalanceOf<T>,
        hosting_org: <T as Org>::OrgId,
        bank_operator: Option<<T as System>::AccountId>,
    ) -> Result<OrgBankAccountOpenedEvent<T>> {
        self.open_org_bank_account(seed, hosting_org, bank_operator)
            .await
    }

    async fn account_posts_bounty(
        &self,
        description: <T as Org>::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>,
        acceptance_committee: ResolutionMetadata<
            <T as Org>::OrgId,
            <T as Vote>::Signal,
            <T as System>::BlockNumber,
        >,
        supervision_committee: Option<
            ResolutionMetadata<
                <T as Org>::OrgId,
                <T as Vote>::Signal,
                <T as System>::BlockNumber,
            >,
        >,
    ) -> Result<BountyPostedEvent<T>> {
        self.account_posts_bounty(
            description,
            amount_reserved_for_bounty,
            acceptance_committee,
            supervision_committee,
        )
        .await
    }

    async fn account_applies_for_bounty(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        description: <T as Org>::IpfsReference,
        total_amount: BalanceOf<T>,
    ) -> Result<BountyApplicationSubmittedEvent<T>> {
        self.account_applies_for_bounty(bounty_id, description, total_amount)
            .await
    }

    async fn account_triggers_application_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        new_grant_app_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationReviewTriggeredEvent<T>> {
        self.account_triggers_application_review(bounty_id, new_grant_app_id)
            .await
    }

    async fn account_sudo_approves_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<SudoApprovedApplicationEvent<T>> {
        self.account_sudo_approves_application(bounty_id, application_id)
            .await
    }

    async fn poll_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationPolledEvent<T>> {
        self.poll_application(bounty_id, application_id).await
    }

    async fn submit_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
        submission_reference: <T as Org>::IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<MilestoneSubmittedEvent<T>> {
        self.submit_milestone(
            bounty_id,
            application_id,
            submission_reference,
            amount_requested,
        )
        .await
    }

    async fn trigger_milestone_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneReviewTriggeredEvent<T>> {
        self.trigger_milestone_review(bounty_id, milestone_id).await
    }

    async fn sudo_approves_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneSudoApprovedEvent<T>> {
        self.sudo_approves_milestone(bounty_id, milestone_id).await
    }

    async fn poll_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestonePolledEvent<T>> {
        self.poll_milestone(bounty_id, milestone_id).await
    }

    fn subxt(&self) -> &substrate_subxt::Client<T> {
        self.subxt()
    }
}
