mod subxt;

pub use subxt::*;

use crate::error::Error;
use async_trait::async_trait;
use codec::Decode;
use substrate_subxt::{
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[async_trait]
pub trait BountyClient<T: Runtime + Bounty>: ChainClient<T> {
    async fn post_bounty(
        &self,
        bounty: T::BountyPost,
        amount: BalanceOf<T>,
    ) -> Result<BountyPostedEvent<T>, Self::Error>;
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>, Self::Error>;
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission: T::BountySubmission,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>, Self::Error>;
    async fn approve_bounty_submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<BountyPaymentExecutedEvent<T>, Self::Error>;
    async fn bounty(
        &self,
        bounty_id: T::BountyId,
    ) -> Result<BountyState<T>, Self::Error>;
    async fn submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<SubState<T>, Self::Error>;
    async fn open_bounties(
        &self,
        min: BalanceOf<T>,
    ) -> Result<Option<Vec<(T::BountyId, BountyState<T>)>>, Self::Error>;
    async fn open_submissions(
        &self,
        bounty_id: T::BountyId,
    ) -> Result<Option<Vec<(T::SubmissionId, SubState<T>)>>, Self::Error>;
}

#[async_trait]
impl<T, C> BountyClient<T> for C
where
    T: Runtime + Bounty,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Bounty>::IpfsReference: From<libipld::cid::Cid>,
    C: ChainClient<T>,
    C::Error: From<Error>,
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
    ) -> Result<BountyPostedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let info = crate::post(self, bounty).await?;
        self.chain_client()
            .post_bounty_and_watch(signer, info.into(), amount)
            .await?
            .bounty_posted()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn contribute_to_bounty(
        &self,
        bounty_id: T::BountyId,
        amount: BalanceOf<T>,
    ) -> Result<BountyRaiseContributionEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .contribute_to_bounty_and_watch(signer, bounty_id, amount)
            .await?
            .bounty_raise_contribution()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn submit_for_bounty(
        &self,
        bounty_id: T::BountyId,
        submission: T::BountySubmission,
        amount: BalanceOf<T>,
    ) -> Result<BountySubmissionPostedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        let submission_ref = crate::post(self, submission).await?;
        self.chain_client()
            .submit_for_bounty_and_watch(
                signer,
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
    ) -> Result<BountyPaymentExecutedEvent<T>, C::Error> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .approve_bounty_submission_and_watch(signer, submission_id)
            .await?
            .bounty_payment_executed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn bounty(
        &self,
        bounty_id: T::BountyId,
    ) -> Result<BountyState<T>, C::Error> {
        Ok(self
            .chain_client()
            .bounties(bounty_id, None)
            .await
            .map_err(Error::Subxt)?)
    }
    async fn submission(
        &self,
        submission_id: T::SubmissionId,
    ) -> Result<SubState<T>, C::Error> {
        Ok(self
            .chain_client()
            .submissions(submission_id, None)
            .await
            .map_err(Error::Subxt)?)
    }
    async fn open_bounties(
        &self,
        min: BalanceOf<T>,
    ) -> Result<Option<Vec<(T::BountyId, BountyState<T>)>>, C::Error> {
        let mut bounties = self
            .chain_client()
            .bounties_iter(None)
            .await
            .map_err(Error::Subxt)?;
        let mut bounties_above_min =
            Vec::<(T::BountyId, BountyState<T>)>::new();
        while let Some((id, bounty)) = bounties.next().await? {
            if bounty.total() >= min {
                let decoded_key = Decode::decode(&mut &id.0[..])?;
                bounties_above_min.push((decoded_key, bounty));
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
    ) -> Result<Option<Vec<(T::SubmissionId, SubState<T>)>>, C::Error> {
        let mut submissions = self
            .chain_client()
            .submissions_iter(None)
            .await
            .map_err(Error::Subxt)?;
        let mut submissions_for_bounty =
            Vec::<(T::SubmissionId, SubState<T>)>::new();
        while let Some((id, submission)) = submissions.next().await? {
            if submission.bounty_id() == bounty_id {
                let decoded_key = Decode::decode(&mut &id.0[..])?;
                submissions_for_bounty.push((decoded_key, submission));
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
    use sunshine_bounty_utils::bounty::BountyInformation;
    use sunshine_core::ChainClient;
    use test_client::{
        bounty::{
            BountyClient,
            BountyPostedEvent,
            BountyRaiseContributionEvent,
        },
        bounty_client::BountyBody,
        mock::{
            test_node,
            AccountKeyring,
        },
        Client,
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
        let (client, _client_tmp) =
            Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        client
            .chain_client()
            .transfer(client.chain_signer().unwrap(), &alice_account_id, 10_000)
            .await
            .unwrap();
    }

    #[async_std::test]
    async fn post_bounty_test() {
        let (node, _node_tmp) = test_node();
        let (client, _client_tmp) =
            Client::mock(&node, AccountKeyring::Alice).await;
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
        let (client, _client_tmp) =
            Client::mock(&node, AccountKeyring::Alice).await;
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
            event1.description,
            alice_account_id.clone(),
            10,
        );
        let expected_bounty2 =
            BountyInformation::new(event2.description, alice_account_id, 10);
        assert_eq!(bounties.get(0).unwrap().1, expected_bounty1);
        assert_eq!(bounties.get(1).unwrap().1, expected_bounty2);
    }

    // #[async_std::test]
    // async fn get_submissions_test() {
    //     let (node, _node_tmp) = test_node();
    //     let (client, _client_tmp) =
    //         Client::mock(&node, AccountKeyring::Alice).await;
    //     let alice_account_id = AccountKeyring::Alice.to_account_id();
    // }

    // #[async_std::test]
    // async fn contribute_to_bounty_test() {
    //     let (node, _node_tmp) = test_node();
    //     let (client, _client_tmp) =
    //         Client::mock(&node, AccountKeyring::Alice).await;
    //     let alice_account_id = AccountKeyring::Alice.to_account_id();
    //     let bounty = BountyBody {
    //         repo_owner: "sunshine-protocol".to_string(),
    //         repo_name: "sunshine-bounty".to_string(),
    //         issue_number: 124,
    //     };
    //     let _ = client.post_bounty(bounty, 10u128).await.unwrap();
    //     let event = client.contribute_to_bounty(1, 5u128).await.unwrap();
    //     let expected_event = BountyRaiseContributionEvent {
    //         contributor: alice_account_id,
    //         amount: 5,
    //         bounty_id: 1,
    //         total: 15,
    //         bounty_ref: event.bounty_ref.clone(),
    //     };
    //     assert_eq!(event, expected_event);
    // }
}
