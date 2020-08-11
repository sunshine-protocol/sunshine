use crate::dto::{
    BountyInformation,
    BountySubmissionInformation,
};
use anyhow::bail;
use ipld_block_builder::{
    Cache,
    Codec,
    ReadonlyCache,
};
use std::marker::PhantomData;
use substrate_subxt::{
    balances::Balances,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    bounty::{
        Bounty as BountyTrait,
        BountyClient,
        BountyState,
        SubState,
    },
    BountyBody,
};
use sunshine_client_utils::{
    cid::CidBytes,
    crypto::{
        bip39::Mnemonic,
        keychain::TypedPair,
        secrecy::{
            ExposeSecret,
            SecretString,
        },
    },
    Keystore,
    Result,
};
use sunshine_ffi_utils::async_std::sync::RwLock;

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

#[derive(Clone, Debug)]
pub struct Key<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    client: &'a RwLock<C>,
    _runtime: PhantomData<R>,
}

impl<'a, C, R> Key<'a, C, R>
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

impl<'a, C, R> Key<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    pub async fn exists(&self) -> Result<bool> {
        self.client.read().await.keystore().is_initialized().await
    }

    pub async fn set(
        &self,
        password: &str,
        suri: Option<&str>,
        paperkey: Option<&str>,
    ) -> Result<String> {
        let password = SecretString::new(password.to_string());
        if password.expose_secret().len() < 8 {
            bail!("Password Too Short");
        }
        let dk = if let Some(paperkey) = paperkey {
            let mnemonic = Mnemonic::parse(paperkey)?;
            TypedPair::<C::KeyType>::from_mnemonic(&mnemonic)?
        } else if let Some(suri) = suri {
            TypedPair::<C::KeyType>::from_suri(suri)?
        } else {
            TypedPair::<C::KeyType>::generate().await
        };

        self.client
            .write()
            .await
            .set_key(dk, &password, false)
            .await?;
        let account_id =
            self.client.read().await.signer()?.account_id().to_string();
        Ok(account_id)
    }

    pub async fn lock(&self) -> Result<bool> {
        self.client.write().await.lock().await?;
        Ok(true)
    }

    pub async fn unlock(&self, password: impl Into<&str>) -> Result<bool> {
        let password = SecretString::new(password.into().to_string());
        self.client.write().await.unlock(&password).await?;
        Ok(true)
    }
}

impl<'a, C, R> Bounty<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
    R: BountyTrait<IpfsReference = CidBytes>,
    C::OffchainClient: Cache<Codec, BountyBody>,
    <R as System>::AccountId: ToString,
    <R as BountyTrait>::BountyId: From<u64> + Into<u64>,
    <R as BountyTrait>::SubmissionId: From<u64> + Into<u64>,
    <R as Balances>::Balance: Into<u128> + From<u64>,
{
    pub async fn get(&self, bounty_id: u64) -> Result<Vec<u8>> {
        let bounty_state =
            self.client.read().await.bounty(bounty_id.into()).await?;

        let info = self.get_bounty_info(bounty_id.into(), bounty_state).await?;
        Ok(serde_cbor::to_vec(&info)?)
    }

    pub async fn post(
        &self,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: u64,
    ) -> Result<u64>
    where
        <R as BountyTrait>::BountyPost: From<BountyBody>,
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
            .await?;
        Ok(event.id.into())
    }

    pub async fn contribute(
        &self,
        bounty_id: u64,
        amount: u64,
    ) -> Result<u128> {
        let event = self
            .client
            .read()
            .await
            .contribute_to_bounty(bounty_id.into(), amount.into())
            .await?;
        Ok(event.total.into())
    }

    pub async fn submit(
        &self,
        bounty_id: u64,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: u64,
    ) -> Result<u64>
    where
        <R as BountyTrait>::BountySubmission: From<BountyBody>,
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
            .await?;
        Ok(event.id.into())
    }

    pub async fn approve(&self, submission_id: u64) -> Result<u128> {
        let event = self
            .client
            .read()
            .await
            .approve_bounty_submission(submission_id.into())
            .await?;
        Ok(event.new_total.into())
    }

    pub async fn get_submission(&self, submission_id: u64) -> Result<Vec<u8>> {
        let submission_state = self
            .client
            .read()
            .await
            .submission(submission_id.into())
            .await?;
        let info = self
            .get_submission_info(submission_id.into(), submission_state)
            .await?;
        Ok(serde_cbor::to_vec(&info)?)
    }

    pub async fn open_bounties(&self, min: u64) -> Result<Vec<u8>> {
        let open_bounties =
            self.client.read().await.open_bounties(min.into()).await?;
        match open_bounties {
            Some(list) => {
                let mut v = Vec::with_capacity(list.len());
                for (id, state) in list {
                    if let Ok(info) = self.get_bounty_info(id, state).await {
                        v.push(info);
                    }
                }
                Ok(serde_cbor::to_vec(&v)?)
            }
            None => Ok(Vec::new()),
        }
    }

    pub async fn open_bounty_submissions(
        &self,
        bounty_id: u64,
    ) -> Result<Vec<u8>> {
        let open_submissions = self
            .client
            .read()
            .await
            .open_submissions(bounty_id.into())
            .await?;
        match open_submissions {
            Some(list) => {
                let mut v = Vec::with_capacity(list.len());
                for (id, state) in list {
                    if let Ok(info) = self.get_submission_info(id, state).await
                    {
                        v.push(info);
                    }
                }
                Ok(serde_cbor::to_vec(&v)?)
            }
            None => Ok(Vec::new()),
        }
    }

    async fn get_bounty_info(
        &self,
        id: <R as BountyTrait>::BountyId,
        state: BountyState<R>,
    ) -> Result<BountyInformation> {
        let event_cid = state.info().to_cid()?;

        let bounty_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await?;

        let info = BountyInformation {
            id: id.into(),
            repo_owner: bounty_body.repo_owner,
            repo_name: bounty_body.repo_name,
            issue_number: bounty_body.issue_number,
            depositer: state.depositer().to_string(),
            total: state.total().into(),
        };
        Ok(info)
    }

    async fn get_submission_info(
        &self,
        id: <R as BountyTrait>::SubmissionId,
        state: SubState<R>,
    ) -> Result<BountySubmissionInformation> {
        let event_cid = state.submission().to_cid()?;

        let submission_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await?;

        let awaiting_review = state.state().awaiting_review();
        let info = BountySubmissionInformation {
            id: id.into(),
            repo_owner: submission_body.repo_owner,
            repo_name: submission_body.repo_name,
            issue_number: submission_body.issue_number,
            bounty_id: state.bounty_id().into(),
            submitter: state.submitter().to_string(),
            amount: state.amount().into(),
            awaiting_review,
            approved: !awaiting_review,
        };
        Ok(info)
    }
}
