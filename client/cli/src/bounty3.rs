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
    bounty3::{
        Bounty3,
        Bounty3Client,
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
    pub async fn exec<R: Runtime + Bounty3, C: Bounty3Client<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty3>::Currency: From<u128> + Display,
        <R as Bounty3>::BountyId: Display,
        <R as Bounty3>::BountyPost: From<BountyBody>,
        <R as Bounty3>::IpfsReference: Display,
    {
        let bounty: <R as Bounty3>::BountyPost = BountyBody {
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
            "AccountId {} posted new bounty with BountyId {} and amount: {} -- the cid is {}",
            event.depositer, event.id, event.amount, event.description
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
    pub async fn exec<R: Runtime + Bounty3, C: Bounty3Client<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty3>::Currency: From<u128> + Display,
        <R as Bounty3>::BountyId: From<u64> + Display,
    {
        let event = client
            .contribute_to_bounty(self.bounty_id.into(), self.amount.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {} contributed to BountyId {} with total contribution of {} s.t. the bounty total is now {}",
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
    pub async fn exec<R: Runtime + Bounty3, C: Bounty3Client<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty3>::Currency: From<u128> + Display,
        <R as Bounty3>::BountyId: From<u64> + Display,
        <R as Bounty3>::SubmissionId: Display,
        <R as Bounty3>::BountySubmission: From<BountyBody>,
        <R as Bounty3>::IpfsReference: Display,
    {
        let bounty: <R as Bounty3>::BountySubmission = BountyBody {
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
            "AccountId {} submitted an entry for Bounty {} requesting amount {} with SubmissionId {} -- the cid is {}",
            event.submitter, event.bounty_id, event.amount, event.id, event.submission_ref
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyApproveCommand {
    pub submission_id: u64,
}

impl BountyApproveCommand {
    pub async fn exec<R: Runtime + Bounty3, C: Bounty3Client<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Bounty3>::Currency: From<u128> + Display,
        <R as Bounty3>::SubmissionId: From<u64> + Display,
        <R as Bounty3>::BountyId: Display,
        <R as Bounty3>::IpfsReference: Display,
    {
        let event = client
            .approve_bounty_submission(self.submission_id.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "BountyId {} was approved (and transferred!) for SubmissionId {} for amount {} -- cid for submission is {}",
            event.bounty_id, event.submission_id, event.amount, event.submission_ref
        );
        Ok(())
    }
}
