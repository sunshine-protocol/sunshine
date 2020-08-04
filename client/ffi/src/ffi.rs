use crate::{
    dto::{
        BountyInformation,
        BountySubmissionInformation,
    },
    error::{
        Error,
        Result,
    },
};
use ffi_utils::async_std::sync::RwLock;
use identity_utils::cid::CidBytes;
use ipld_block_builder::{
    Cache,
    Codec,
    ReadonlyCache,
};
use std::marker::PhantomData;
use substrate_subxt::{
    system::System,
    Runtime,
};
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
    R: BountyTrait<IpfsReference = CidBytes>,
{
    pub async fn get(&self, bounty_id: u64) -> Result<Vec<u8>, C::Error>
    where
        <R as BountyTrait>::BountyId: From<u64>,
        <R as System>::AccountId: ToString,
        <R as BountyTrait>::Currency: Into<u128>,
        C::OffchainClient: Cache<Codec, BountyBody>,
    {
        let bounty_state = self
            .client
            .read()
            .await
            .bounty(bounty_id.into())
            .await
            .map_err(Error::Client)?;

        let event_cid =
            bounty_state.info().to_cid().map_err(Error::CiDecode)?;

        let bounty_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;
        let info = BountyInformation {
            repo_owner: bounty_body.repo_owner,
            repo_name: bounty_body.repo_name,
            issue_number: bounty_body.issue_number,
            depositer: bounty_state.depositer().to_string(),
            total: bounty_state.total().into(),
        };
        serde_cbor::to_vec(&info).map_err(Error::Cbor)
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
    ) -> Result<Vec<u8>, C::Error>
    where
        <R as BountyTrait>::SubmissionId: From<u64>,
        <R as BountyTrait>::BountyId: Into<u64>,
        <R as System>::AccountId: ToString,
        <R as BountyTrait>::Currency: Into<u128>,
        <R as BountyTrait>::Currency: Into<u128>,
        C::OffchainClient: Cache<Codec, BountyBody>,
    {
        let submission_state = self
            .client
            .read()
            .await
            .submission(submission_id.into())
            .await
            .map_err(Error::Client)?;
        let event_cid = submission_state
            .submission()
            .to_cid()
            .map_err(Error::CiDecode)?;

        let submission_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await
            .map_err(Error::Libipld)?;

        let awaiting_review = submission_state.awaiting_review();
        let info = BountySubmissionInformation {
            repo_owner: submission_body.repo_owner,
            repo_name: submission_body.repo_name,
            issue_number: submission_body.issue_number,
            bounty_id: submission_state.bounty_id().into(),
            submitter: submission_state.submitter().to_string(),
            amount: submission_state.amount().into(),
            awaiting_review,
            approved: !awaiting_review,
        };
        serde_cbor::to_vec(&info).map_err(Error::Cbor)
    }
}
