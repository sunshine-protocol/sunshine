use crate::error::{
    Error,
    Result,
};
use ffi_utils::async_std::sync::RwLock;
use std::marker::PhantomData;
use substrate_subxt::Runtime;
use sunshine_bounty_client::{
    bounty::{
        Bounty as BountyTrait,
        BountyClient,
    },
    BountyBody,
};

#[derive(Clone, Debug)]
pub struct Bounty<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    client: &'a RwLock<C>,
    _runtime: PhantomData<R>,
}

impl<'a, C, R> Bounty<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    pub fn new(client: &'a RwLock<C>) -> Self {
        Self {
            client,
            _runtime: PhantomData,
        }
    }
}

impl<'a, C, R> Bounty<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    pub async fn get(&self, bounty_id: u64) -> Result<Vec<String>, C::Error>
    where
        <R as BountyTrait>::BountyId: From<u64>,
        <R as BountyTrait>::Currency: ToString,
    {
        let bounty_state = self
            .client
            .read()
            .await
            .bounty(bounty_id.into())
            .await
            .map_err(Error::Client)?;
        Ok(vec![
            bounty_state.depositer().to_string(),
            bounty_state.total().to_string(),
        ])
    }

    pub async fn post(
        &self,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: u64,
    ) -> Result<u64, C::Error>
    where
        <R as BountyTrait>::BountyPost: From<BountyBody>,
        <R as BountyTrait>::Currency: From<u64>,
        <R as BountyTrait>::BountyId: Into<u64>,
    {
        let bounty = BountyBody {
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
            issue_number,
        }
        .into();
        let event = self
            .client
            .read()
            .await
            .post_bounty(bounty, amount.into())
            .await
            .map_err(Error::Client)?;
        Ok(event.id.into())
    }

    pub async fn contribute(
        &self,
        bounty_id: u64,
        amount: u64,
    ) -> Result<u128, C::Error>
    where
        <R as BountyTrait>::Currency: From<u64> + Into<u128>,
        <R as BountyTrait>::BountyId: From<u64>,
    {
        let event = self
            .client
            .read()
            .await
            .contribute_to_bounty(bounty_id.into(), amount.into())
            .await
            .map_err(Error::Client)?;
        Ok(event.total.into())
    }

    pub async fn submit(
        &self,
        bounty_id: u64,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: u64,
    ) -> Result<u64, C::Error>
    where
        <R as BountyTrait>::BountySubmission: From<BountyBody>,
        <R as BountyTrait>::Currency: From<u64>,
        <R as BountyTrait>::BountyId: From<u64>,
        <R as BountyTrait>::SubmissionId: Into<u64>,
    {
        let bounty = BountyBody {
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
            issue_number,
        }
        .into();
        let event = self
            .client
            .read()
            .await
            .submit_for_bounty(bounty_id.into(), bounty, amount.into())
            .await
            .map_err(Error::Client)?;
        Ok(event.id.into())
    }

    pub async fn approve(&self, submission_id: u64) -> Result<u128, C::Error>
    where
        <R as BountyTrait>::Currency: Into<u128>,
        <R as BountyTrait>::SubmissionId: From<u64>,
    {
        let event = self
            .client
            .read()
            .await
            .approve_bounty_submission(submission_id.into())
            .await
            .map_err(Error::Client)?;
        Ok(event.new_total.into())
    }

    pub async fn get_submission(
        &self,
        submission_id: u64,
    ) -> Result<Vec<String>, C::Error>
    where
        <R as BountyTrait>::SubmissionId: From<u64>,
        <R as BountyTrait>::Currency: ToString,
    {
        let submission_state = self
            .client
            .read()
            .await
            .submission(submission_id.into())
            .await
            .map_err(Error::Client)?;
        Ok(vec![submission_state.amount().to_string()])
    }
}
