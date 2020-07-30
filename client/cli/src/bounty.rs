use crate::error::{
    Error,
    Result,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    bounty::{
        Bounty,
        BountyClient,
    },
    BountyBody,
};

#[derive(Clone, Debug, Clap)]
pub struct BountyPostCommand {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub amount: u128,
}

impl BountyPostCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: Display,
        <R as Bounty>::BountyPost: From<BountyBody>,
    {
        let bounty: <R as Bounty>::BountyPost = BountyBody {
            repo_owner: (*self.repo_owner).to_string(),
            repo_name: (*self.repo_name).to_string(),
            issue_number: self.issue_number,
        }
        .into();
        let event = client
            .post_bounty(bounty, self.amount.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "Depositer with AccountId {} posted new BountyId {}, Balance {}",
            event.depositer, event.id, event.amount,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyContributeCommand {
    pub bounty_id: u64,
    pub amount: u128,
}

impl BountyContributeCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .contribute_to_bounty(self.bounty_id.into(), self.amount.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "Contributor with AccountId {} contributed to BountyId {} s.t. their total contribution is {} and the Total Balance for the Bounty is now {}",
            event.contributor, event.bounty_id, event.new_amount, event.total
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountySubmitCommand {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
    pub bounty_id: u64,
    pub amount: u128,
}

impl BountySubmitCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty>::Currency: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
        <R as Bounty>::SubmissionId: Display,
        <R as Bounty>::BountySubmission: From<BountyBody>,
    {
        let bounty: <R as Bounty>::BountySubmission = BountyBody {
            repo_owner: (*self.repo_owner).to_string(),
            repo_name: (*self.repo_name).to_string(),
            issue_number: self.issue_number,
        }
        .into();
        let event = client
            .submit_for_bounty(
                self.bounty_id.into(),
                bounty,
                self.amount.into(),
            )
            .await
            .map_err(Error::Client)?;
        println!(
            "Submitter with AccountId {} submitted for BountyId {}, requesting Balance {} with SubmissionId {:?}",
            event.submitter, event.bounty_id, event.amount, event.id,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyApproveCommand {
    pub submission_id: u64,
}

impl BountyApproveCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty>::Currency: From<u128> + Display,
        <R as Bounty>::SubmissionId: From<u64> + Display,
        <R as Bounty>::BountyId: Display,
    {
        let event = client
            .approve_bounty_submission(self.submission_id.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "Approved SubmissionId {} to transfer Balance {} to AccountId {}. Remaining Balance {} for BountyId {} ",
            event.submission_id, event.amount, event.submitter, event.new_total, event.bounty_id
        );
        Ok(())
    }
}
