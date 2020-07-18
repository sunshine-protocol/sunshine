mod subxt;

pub use subxt::*;
pub use sunshine_bounty_utils::bounty::*;

use crate::{
    bank::BalanceOf,
    error::Error,
    org::Org,
    vote::Vote,
    BountyBody,
    TextBlock,
};
use async_trait::async_trait;
use substrate_subxt::{
    system::System,
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_bounty_utils::court::ResolutionMetadata;
use sunshine_core::ChainClient;

#[async_trait]
pub trait BountyClient<T: Runtime + Bounty>: ChainClient<T> {
    async fn account_posts_bounty(
        &self,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
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
    ) -> Result<BountyPostedEvent<T>, Self::Error>;
    async fn account_applies_for_bounty(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        description: String,
        total_amount: BalanceOf<T>,
    ) -> Result<BountyApplicationSubmittedEvent<T>, Self::Error>;
    async fn account_triggers_application_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        new_grant_app_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationReviewTriggeredEvent<T>, Self::Error>;
    async fn account_sudo_approves_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<SudoApprovedApplicationEvent<T>, Self::Error>;
    async fn poll_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationPolledEvent<T>, Self::Error>;
    async fn submit_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
        amount_requested: BalanceOf<T>,
    ) -> Result<MilestoneSubmittedEvent<T>, Self::Error>;
    async fn trigger_milestone_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneReviewTriggeredEvent<T>, Self::Error>;
    async fn sudo_approves_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneSudoApprovedEvent<T>, Self::Error>;
    async fn poll_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestonePolledEvent<T>, Self::Error>;
}

#[async_trait]
impl<T, C> BountyClient<T> for C
where
    T: Runtime + Bounty,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::IpfsReference: From<libipld::cid::Cid>,
    C: ChainClient<T>,
    C::Error: From<Error>,
    C::OffchainClient: ipld_block_builder::Cache<ipld_block_builder::Codec, TextBlock>
        + ipld_block_builder::Cache<ipld_block_builder::Codec, BountyBody>,
{
    async fn account_posts_bounty(
        &self,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
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
    ) -> Result<BountyPostedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let description = crate::post(
            self,
            BountyBody {
                repo_owner,
                repo_name,
                issue_number,
            },
        )
        .await?;
        self.chain_client()
            .account_posts_bounty_and_watch(
                signer,
                description,
                amount_reserved_for_bounty,
                acceptance_committee,
                supervision_committee,
            )
            .await?
            .bounty_posted()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn account_applies_for_bounty(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        description: String,
        total_amount: BalanceOf<T>,
    ) -> Result<BountyApplicationSubmittedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let description =
            crate::post(self, TextBlock { text: description }).await?;
        self.chain_client()
            .account_applies_for_bounty_and_watch(
                signer,
                bounty_id,
                description,
                total_amount,
            )
            .await?
            .bounty_application_submitted()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn account_triggers_application_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        new_grant_app_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationReviewTriggeredEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .account_triggers_application_review_and_watch(
                signer,
                bounty_id,
                new_grant_app_id,
            )
            .await?
            .application_review_triggered()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn account_sudo_approves_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<SudoApprovedApplicationEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .account_sudo_approves_application_and_watch(
                signer,
                bounty_id,
                application_id,
            )
            .await?
            .sudo_approved_application()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn poll_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationPolledEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .poll_application_and_watch(signer, bounty_id, application_id)
            .await?
            .application_polled()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn submit_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
        repo_owner: String,
        repo_name: String,
        issue_number: u64,
        amount_requested: BalanceOf<T>,
    ) -> Result<MilestoneSubmittedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let submission_reference = crate::post(
            self,
            BountyBody {
                repo_owner,
                repo_name,
                issue_number,
            },
        )
        .await?;
        self.chain_client()
            .submit_milestone_and_watch(
                signer,
                bounty_id,
                application_id,
                submission_reference,
                amount_requested,
            )
            .await?
            .milestone_submitted()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn trigger_milestone_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneReviewTriggeredEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .trigger_milestone_review_and_watch(signer, bounty_id, milestone_id)
            .await?
            .milestone_review_triggered()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn sudo_approves_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneSudoApprovedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .sudo_approves_milestone_and_watch(signer, bounty_id, milestone_id)
            .await?
            .milestone_sudo_approved()?
            .ok_or(Error::EventNotFound.into())
    }
    async fn poll_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestonePolledEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .poll_milestone_and_watch(signer, bounty_id, milestone_id)
            .await?
            .milestone_polled()?
            .ok_or(Error::EventNotFound.into())
    }
}
