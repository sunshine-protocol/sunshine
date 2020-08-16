use crate::{
    dto::{
        BountyInformation,
        BountySubmissionInformation,
    },
    ffi_utils::log::{
        error,
        info,
        warn,
    },
};
use anyhow::{
    anyhow,
    bail,
};
use ipld_block_builder::{
    Cache,
    Codec,
    ReadonlyCache,
};
use std::{
    fmt::{
        Debug,
        Display,
    },
    marker::PhantomData,
};
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
        TransferCallExt,
        TransferEventExt,
    },
    sp_core::crypto::Ss58Codec,
    system::{
        AccountStoreExt,
        System,
    },
    Runtime,
    SignedExtension,
    SignedExtra,
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
        ss58::Ss58,
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

#[derive(Clone, Debug)]
pub struct Wallet<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + BountyTrait,
{
    client: &'a RwLock<C>,
    _runtime: PhantomData<R>,
}

impl<'a, C, R> Wallet<'a, C, R>
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

    pub async fn uid(&self) -> Result<String> {
        let client = self.client.read().await;
        let signer = client.signer()?;
        Ok(signer.account_id().to_string())
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
    R: Runtime + BountyTrait + Debug,
    R: BountyTrait<IpfsReference = CidBytes>,
    C::OffchainClient: Cache<Codec, BountyBody>,
    <R as System>::AccountId: Ss58Codec + Into<<R as System>::Address>,
    <R as BountyTrait>::BountyId: From<u64> + Into<u64> + Display,
    <R as BountyTrait>::SubmissionId: From<u64> + Into<u64> + Display,
    <R as BountyTrait>::BountyPost: From<BountyBody> + Debug,
    <R as BountyTrait>::BountySubmission: From<BountyBody> + Debug,
    <R as Balances>::Balance: Into<u128> + From<u64>,
{
    pub async fn get(&self, bounty_id: &str) -> Result<String> {
        info!("Getting Bounty with id: {}", bounty_id);
        let bounty_state = self
            .client
            .read()
            .await
            .bounty(bounty_id.parse::<u64>()?.into())
            .await?;
        info!("Got bounty State for BountyId: {}", bounty_id);
        let info = self
            .get_bounty_info(bounty_id.parse::<u64>()?.into(), bounty_state)
            .await?;
        info!("Bounty Info: {:?}", info);
        Ok(serde_json::to_string(&info)?)
    }

    pub async fn post(
        &self,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: &str,
    ) -> Result<u64> {
        let bounty = BountyBody {
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
            issue_number,
        }
        .into();
        info!("Posting Bounty: {:?}", bounty);
        let event = self
            .client
            .read()
            .await
            .post_bounty(bounty, amount.parse::<u64>()?.into())
            .await?;
        info!("Bounty Created: {:?}", event);
        Ok(event.id.into())
    }

    pub async fn contribute(
        &self,
        bounty_id: &str,
        amount: &str,
    ) -> Result<u128> {
        info!("Contribute to BountyId: {}", bounty_id);
        let event = self
            .client
            .read()
            .await
            .contribute_to_bounty(
                bounty_id.parse::<u64>()?.into(),
                amount.parse::<u64>()?.into(),
            )
            .await?;
        info!("Contibution Added: {:?}", event);
        Ok(event.total.into())
    }

    pub async fn submit(
        &self,
        bounty_id: &str,
        repo_owner: &str,
        repo_name: &str,
        issue_number: u64,
        amount: &str,
    ) -> Result<u64> {
        let bounty = BountyBody {
            repo_owner: repo_owner.to_string(),
            repo_name: repo_name.to_string(),
            issue_number,
        }
        .into();
        info!("Submit for BountyId: {} with {:?}", bounty_id, bounty);
        let event = self
            .client
            .read()
            .await
            .submit_for_bounty(
                bounty_id.parse::<u64>()?.into(),
                bounty,
                amount.parse::<u64>()?.into(),
            )
            .await?;
        info!("Submission Added: {:?}", event);
        Ok(event.id.into())
    }

    pub async fn approve(&self, submission_id: &str) -> Result<u128> {
        info!("Approving SubmissionId: {}", submission_id);
        let event = self
            .client
            .read()
            .await
            .approve_bounty_submission(submission_id.parse::<u64>()?.into())
            .await?;
        info!("Approved SubmissionId: {} with {:?}", submission_id, event);
        Ok(event.new_total.into())
    }

    pub async fn get_submission(&self, submission_id: &str) -> Result<String> {
        info!("Getting SubmissionId: {}", submission_id);
        let submission_state = self
            .client
            .read()
            .await
            .submission(submission_id.parse::<u64>()?.into())
            .await?;
        info!("Got Submission State: {:?}", submission_state);
        let info = self
            .get_submission_info(
                submission_id.parse::<u64>()?.into(),
                submission_state,
            )
            .await?;
        info!("Submission: {:?}", info);
        Ok(serde_json::to_string(&info)?)
    }

    pub async fn get_contribution(
        &self,
        acc: &str,
        bounty_id: &str,
    ) -> Result<String> {
        let account = acc.parse::<Ss58<R>>()?;
        info!(
            "Getting the contribution for Account {} in Bounty {}",
            account.0, bounty_id
        );
        let amount = self
            .client
            .read()
            .await
            .contribution(bounty_id.parse::<u64>()?.into(), account.0)
            .await?;
        info!("Contributed Balance: {:?}", amount.total());
        Ok(serde_json::to_string(&amount.total())?)
    }

    pub async fn open_bounties(&self, min: &str) -> Result<String> {
        info!("Getting Open Bounties with min: {}", min);
        let open_bounties = self
            .client
            .read()
            .await
            .open_bounties(min.parse::<u64>()?.into())
            .await?;
        info!("is there any Open Bounties? {}", open_bounties.is_some());
        match open_bounties {
            Some(list) => {
                let mut v = Vec::with_capacity(list.len());
                for (id, state) in list {
                    info!("Listing Bounty #{} with State: {:?}", id, state);
                    match self.get_bounty_info(id, state).await {
                        Ok(info) => {
                            info!("Adding it to the list: {:?}", info);
                            v.push(info);
                        }
                        Err(e) => {
                            warn!("I can't get the info of Bounty #{}. Skipping...", id);
                            error!("{:?}", e);
                        }
                    }
                }
                Ok(serde_json::to_string(&v)?)
            }
            None => {
                info!("Empty, No Open Bounties");
                Ok(String::new())
            }
        }
    }

    pub async fn open_bounty_submissions(
        &self,
        bounty_id: &str,
    ) -> Result<String> {
        info!("Getting Open Submissions for BountyId: {}", bounty_id);
        let open_submissions = self
            .client
            .read()
            .await
            .open_submissions(bounty_id.parse::<u64>()?.into())
            .await?;
        info!(
            "is there any Open Submissions? {}",
            open_submissions.is_some()
        );
        match open_submissions {
            Some(list) => {
                let mut v = Vec::with_capacity(list.len());
                for (id, state) in list {
                    info!("Listing Submission #{} with State: {:?}", id, state);
                    match self.get_submission_info(id, state).await {
                        Ok(info) => {
                            info!("Adding it to the list: {:?}", info);
                            v.push(info);
                        }
                        Err(e) => {
                            warn!("I can't get the info of Submission #{}. Skipping..", id);
                            error!("{:?}", e);
                        }
                    }
                }
                Ok(serde_json::to_string(&v)?)
            }
            None => Ok(String::new()),
        }
    }

    async fn get_bounty_info(
        &self,
        id: <R as BountyTrait>::BountyId,
        state: BountyState<R>,
    ) -> Result<BountyInformation> {
        info!("Get bounty info of id: {}", id);
        let event_cid = state.info().to_cid()?;
        let bounty_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await?;
        info!("Bounty Body: {:?}", bounty_body);
        let info = BountyInformation {
            id: id.to_string(),
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
        info!("Get submission info of id: {}", id);
        let event_cid = state.submission().to_cid()?;

        let submission_body: BountyBody = self
            .client
            .read()
            .await
            .offchain_client()
            .get(&event_cid)
            .await?;
        info!("Submission Body: {:?}", submission_body);
        let awaiting_review = state.state().awaiting_review();
        let info = BountySubmissionInformation {
            id: id.to_string(),
            repo_owner: submission_body.repo_owner,
            repo_name: submission_body.repo_name,
            issue_number: submission_body.issue_number,
            bounty_id: state.bounty_id().to_string(),
            submitter: state.submitter().to_string(),
            amount: state.amount().into(),
            awaiting_review,
            approved: !awaiting_review,
        };
        Ok(info)
    }
}

impl<'a, C, R> Wallet<'a, C, R>
where
    C: BountyClient<R> + Send + Sync,
    R: Runtime + Balances + BountyTrait,
    R: System<AccountData = AccountData<<R as Balances>::Balance>>,
    <R as Balances>::Balance: Into<u128> + From<u64>,
    <R as System>::AccountId: Ss58Codec + Into<<R as System>::Address>,
    <<<R as Runtime>::Extra as SignedExtra<R>>::Extra as SignedExtension>::AdditionalSigned: Send + Sync,


{
    pub async fn balance(&self, identifier: Option<&str>) -> Result<R::Balance> {
        let client = self.client.read().await;
        let account_id: Ss58<R> = if let Some(identifier) = identifier {
            identifier.parse()?
        } else {
            Ss58(client.signer()?.account_id().clone())
        };
        let account = client.chain_client().account(&account_id.0, None).await?;
        Ok(account.data.free)
    }

    pub async fn transfer(
        &self,
        to: &str,
        amount: u64,
    ) -> Result<R::Balance> {
        let client = self.client.read().await;
        let account_id: Ss58<R> = to.parse()?;
        let signer = client.chain_signer()?;
        client
            .chain_client()
            .transfer_and_watch(&signer, &account_id.0.into(), amount.into())
            .await?
            .transfer()
            .map_err(|_| anyhow!("Failed to decode transfer event"))?
            .ok_or_else(|| anyhow!("Failed to find transfer event"))?;
        self.balance(None).await
    }
}
