mod subxt;
mod utils;

pub use subxt::*;
pub use utils::AccountShare;

use crate::error::Error;
use substrate_subxt::{
    system::System,
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
pub trait OrgClient<T: Runtime + Org>: Client<T> {
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>>;
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
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
}

#[async_trait]
impl<T, C> OrgClient<T> for C
where
    T: Runtime + Org,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::IpfsReference: From<libipld::cid::Cid>,
    C: Client<T>,
    C::OffchainClient: ipld_block_builder::Cache<
        ipld_block_builder::Codec,
        <T as Org>::Constitution,
    >,
{
    async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>> {
        let signer = self.chain_signer()?;
        let constitution = crate::post(self, constitution).await?;
        self.chain_client()
            .register_flat_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution.into(),
                members,
            )
            .await?
            .new_flat_organization_registered()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>> {
        let signer = self.chain_signer()?;
        let constitution = crate::post(self, constitution).await?;
        self.chain_client()
            .register_weighted_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution.into(),
                weighted_members,
            )
            .await?
            .new_weighted_organization_registered()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .issue_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .burn_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_issue_shares_and_watch(&signer, organization, new_accounts)
            .await?
            .shares_batch_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_burn_shares_and_watch(&signer, organization, old_accounts)
            .await?
            .shares_batch_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .reserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_reserved()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .unreserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_un_reserved()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .lock_shares_and_watch(&signer, org, who)
            .await?
            .shares_locked()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .unlock_shares_and_watch(&signer, org, who)
            .await?
            .shares_unlocked()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
}

#[cfg(test)]
mod tests {
    use rand::{
        rngs::OsRng,
        RngCore,
    };
    use sunshine_core::ChainClient;
    use test_client::{
        bounty_client::TextBlock,
        mock::{
            test_node,
            AccountKeyring,
        },
        org::{
            NewFlatOrganizationRegisteredEvent,
            OrgClient,
        },
        Client,
    };

    // For testing purposes only, NEVER use this to generate AccountIds in practice because it's random
    pub fn random_account_id() -> substrate_subxt::sp_runtime::AccountId32 {
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
    async fn register_flat_org_test() {
        let (node, _node_tmp) = test_node();
        let (client, _client_tmp) =
            Client::mock(&node, AccountKeyring::Alice).await;
        let alice_account_id = AccountKeyring::Alice.to_account_id();
        // insert constitution into
        let raw_const = TextBlock {
            text: "good code lives forever".to_string(),
        };
        let (two, three, four, five, six, seven) = (
            random_account_id(),
            random_account_id(),
            random_account_id(),
            random_account_id(),
            random_account_id(),
            random_account_id(),
        );
        let members =
            vec![alice_account_id.clone(), two, three, four, five, six, seven];
        let event = client
            .register_flat_org(
                Some(alice_account_id.clone()),
                None,
                raw_const,
                &members,
            )
            .await
            .unwrap();
        let expected_event = NewFlatOrganizationRegisteredEvent {
            caller: alice_account_id,
            new_id: 2,
            constitution: event.constitution.clone(),
            total: 7,
        };
        assert_eq!(event, expected_event);
    }
}
