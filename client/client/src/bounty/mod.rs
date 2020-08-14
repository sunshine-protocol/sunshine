mod subxt;

pub use subxt::*;

use crate::error::Error;
use substrate_subxt::{
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_client_utils::{
    async_trait,
    Client,
    Result,
};

#[async_trait]
pub trait BountyClient<T: Runtime + Bounty>: Client<T> {
    async fn post_bounty(
        &self,
        bounty: T::BountyPost,
        amount: BalanceOf<T>,
    ) -> Result<BountyPostedEvent<T>>;
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>>;
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission: T::BountySubmission,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>>;
    async fn approve_bounty_submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<BountyPaymentExecutedEvent<T>>;
    async fn bounty(&self, bounty_id: T::BountyId) -> Result<BountyState<T>>;
    async fn submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<SubState<T>>;
    async fn open_bounties(
        &self,
        min: BalanceOf<T>,
    ) -> Result<Option<Vec<(T::BountyId, BountyState<T>)>>>;
    async fn open_submissions(
        &self,
        bounty_id: T::BountyId,
    ) -> Result<Option<Vec<(T::SubmissionId, SubState<T>)>>>;
}

#[async_trait]
impl<T, C> BountyClient<T> for C
where
    T: Runtime + Bounty,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Bounty>::IpfsReference: From<libipld::cid::Cid>,
    C: Client<T>,
    C::OffchainClient: ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Bounty>::BountyPost,
        > + ipld_block_builder::Cache<
            ipld_block_builder::Codec,
            <T as Bounty>::BountySubmission,
        >,
{
    async fn post_bounty(
        &self,
        bounty: T::BountyPost,
        amount: BalanceOf<T>,
    ) -> Result<BountyPostedEvent<T>> {
        let signer = self.chain_signer()?;
        let info = crate::post(self, bounty).await?;
        self.chain_client()
            .post_bounty_and_watch(&signer, info.into(), amount)
            .await?
            .bounty_posted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .contribute_to_bounty_and_watch(&signer, bounty_id, amount)
            .await?
            .bounty_raise_contribution()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission: T::BountySubmission,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>> {
        let signer = self.chain_signer()?;
        let submission_ref = crate::post(self, submission).await?;
        self.chain_client()
            .submit_for_bounty_and_watch(
                &signer,
                bounty_id,
                submission_ref.into(),
                amount,
            )
            .await?
            .bounty_submission_posted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn approve_bounty_submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<BountyPaymentExecutedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .approve_bounty_submission_and_watch(&signer, submission_id)
            .await?
            .bounty_payment_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn bounty(&self, bounty_id: T::BountyId) -> Result<BountyState<T>> {
        Ok(self.chain_client().bounties(bounty_id, None).await?)
    }
    async fn submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<SubState<T>> {
        Ok(self.chain_client().submissions(submission_id, None).await?)
    }
    async fn open_bounties(
        &self,
        min: BalanceOf<T>,
    ) -> Result<Option<Vec<(T::BountyId, BountyState<T>)>>> {
        let mut bounties = self.chain_client().bounties_iter(None).await?;
        let mut bounties_above_min =
            Vec::<(T::BountyId, BountyState<T>)>::new();
        while let Some((_, bounty)) = bounties.next().await? {
            if bounty.total() >= min {
                bounties_above_min.push((bounty.id(), bounty));
            }
        }
        if bounties_above_min.is_empty() {
            Ok(None)
        } else {
            Ok(Some(bounties_above_min))
        }
    }
    async fn open_submissions(
        &self,
        bounty_id: T::BountyId,
    ) -> Result<Option<Vec<(T::SubmissionId, SubState<T>)>>> {
        let mut submissions =
            self.chain_client().submissions_iter(None).await?;
        let mut submissions_for_bounty =
            Vec::<(T::SubmissionId, SubState<T>)>::new();
        while let Some((_, submission)) = submissions.next().await? {
            if submission.bounty_id() == bounty_id {
                submissions_for_bounty
                    .push((submission.submission_id(), submission));
            }
        }
        if submissions_for_bounty.is_empty() {
            Ok(None)
        } else {
            Ok(Some(submissions_for_bounty))
        }
    }
}

#[cfg(test)]
mod tests {
    use rand::{
        rngs::OsRng,
        RngCore,
    };
    use test_client::{
        bounty::{
            BountyClient,
            BountyPostedEvent,
            BountyRaiseContributionEvent,
        },
        client::Client as _,
        mock::{
            test_node,
            AccountKeyring,
            Client,
        },
        utils::bounty::BountyInformation,
        BountyBody,
    };

    // For testing purposes only, NEVER use this to generate AccountIds in practice because it's random
    pub fn _random_account_id() -> substrate_subxt::sp_runtime::AccountId32 {
        let mut buf = [0u8; 32];
        OsRng.fill_bytes(&mut buf);
        buf.into()
    }

    #[async_std::test]
    async fn simple_test() {
        use substrate_subxt::balances::TransferCallExt;
        let (node, _node_tmp) = test_node();
        let client = Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        client
            .chain_client()
            .transfer(
                &client.chain_signer().unwrap(),
                &alice_account_id,
                10_000,
            )
            .await
            .unwrap();
    }

    #[async_std::test]
    async fn post_bounty_test() {
        let (node, _node_tmp) = test_node();
        let client = Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        let bounty = BountyBody {
            repo_owner: "sunshine-protocol".to_string(),
            repo_name: "sunshine-bounty".to_string(),
            issue_number: 124,
        };
        let event = client.post_bounty(bounty, 10u128).await.unwrap();
        let expected_event = BountyPostedEvent {
            depositer: alice_account_id,
            amount: 10,
            id: 1,
            description: event.description.clone(),
        };
        assert_eq!(event, expected_event);
    }

    #[async_std::test]
    async fn get_bounties_test() {
        let (node, _node_tmp) = test_node();
        let client = Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        let bounty1 = BountyBody {
            repo_owner: "sunshine-protocol".to_string(),
            repo_name: "sunshine-bounty".to_string(),
            issue_number: 124,
        };
        let event1 = client.post_bounty(bounty1, 10u128).await.unwrap();
        let bounty2 = BountyBody {
            repo_owner: "sunshine-protocol".to_string(),
            repo_name: "sunshine-bounty".to_string(),
            issue_number: 124,
        };
        let event2 = client.post_bounty(bounty2, 10u128).await.unwrap();
        let bounties = client.open_bounties(9u128).await.unwrap().unwrap();
        assert_eq!(bounties.len(), 2);
        let expected_bounty1 = BountyInformation::new(
            1u64,
            event1.description,
            alice_account_id.clone(),
            10,
        );
        let expected_bounty2 = BountyInformation::new(
            2u64,
            event2.description,
            alice_account_id,
            10,
        );
        assert_eq!(bounties.get(0).unwrap().1, expected_bounty2);
        assert_eq!(bounties.get(0).unwrap().0, 2u64);
        assert_eq!(bounties.get(1).unwrap().1, expected_bounty1);
        assert_eq!(bounties.get(1).unwrap().0, 1u64);
    }

    #[async_std::test]
    async fn contribute_to_bounty_test() {
        use substrate_subxt::system::AccountStoreExt;
        env_logger::try_init().ok();
        let (node, _node_tmp) = test_node();
        let client = Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        let bounty = BountyBody {
            repo_owner: "sunshine-protocol".to_string(),
            repo_name: "sunshine-bounty".to_string(),
            issue_number: 124,
        };

        let b = client
            .chain_client()
            .account(&alice_account_id, None)
            .await
            .unwrap()
            .data
            .free;
        println!("{}", b);

        let event1 = client.post_bounty(bounty, 1000).await.unwrap();
        let expected_event1 = BountyPostedEvent {
            depositer: alice_account_id.clone(),
            amount: 1000,
            id: 1,
            description: event1.description.clone(),
        };
        assert_eq!(event1, expected_event1);

        let b = client
            .chain_client()
            .account(&alice_account_id, None)
            .await
            .unwrap()
            .data
            .free;
        println!("{}", b);

        let event2 = client.contribute_to_bounty(1, 1000).await.unwrap();
        let expected_event2 = BountyRaiseContributionEvent {
            contributor: alice_account_id.clone(),
            amount: 1000,
            bounty_id: 1,
            total: 2000,
            bounty_ref: event2.bounty_ref.clone(),
        };
        assert_eq!(event2, expected_event2);

        let b = client
            .chain_client()
            .account(&alice_account_id, None)
            .await
            .unwrap()
            .data
            .free;
        println!("{}", b);
    }
}
