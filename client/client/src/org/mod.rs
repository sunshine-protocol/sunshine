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
    async fn new_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrgEvent<T>>;
    async fn new_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrgEvent<T>>;
    async fn issue_shares(
        &self,
        org: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>>;
    async fn burn_shares(
        &self,
        org: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>>;
    async fn batch_issue_shares(
        &self,
        org: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>>;
    async fn batch_burn_shares(
        &self,
        org: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>>;
    async fn org_parent_child(
        &self,
        parent: <T as Org>::OrgId,
        child: <T as Org>::OrgId,
    ) -> bool;
    async fn org(&self, org: <T as Org>::OrgId) -> Result<OrgState<T>>;
    async fn share_profile(
        &self,
        org: <T as Org>::OrgId,
        account: <T as System>::AccountId,
    ) -> Result<Prof<T>>;
    async fn org_relations(&self) -> Result<Vec<Relacion<T>>>;
    async fn org_members(
        &self,
        org: <T as Org>::OrgId,
    ) -> Result<Option<Vec<(T::AccountId, Prof<T>)>>>;
    async fn share_profiles(
        &self,
        account: <T as System>::AccountId,
    ) -> Result<Option<Vec<(T::OrgId, Prof<T>, OrgState<T>)>>>;
}

#[async_trait]
impl<T, C> OrgClient<T> for C
where
    T: Runtime + Org,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Org>::Cid: From<libipld::cid::Cid>,
    C: Client<T>,
    C::OffchainClient: ipld_block_builder::Cache<
        ipld_block_builder::Codec,
        <T as Org>::Constitution,
    >,
{
    async fn new_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrgEvent<T>> {
        let signer = self.chain_signer()?;
        let constitution = crate::post(self, constitution).await?;
        self.chain_client()
            .new_flat_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution.into(),
                members,
            )
            .await?
            .new_flat_org()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn new_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::Constitution,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrgEvent<T>> {
        let signer = self.chain_signer()?;
        let constitution = crate::post(self, constitution).await?;
        self.chain_client()
            .new_weighted_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution.into(),
                weighted_members,
            )
            .await?
            .new_weighted_org()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn issue_shares(
        &self,
        org: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .issue_shares_and_watch(&signer, org, &who, shares)
            .await?
            .shares_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn burn_shares(
        &self,
        org: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .burn_shares_and_watch(&signer, org, &who, shares)
            .await?
            .shares_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_issue_shares(
        &self,
        org: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_issue_shares_and_watch(&signer, org, new_accounts)
            .await?
            .shares_batch_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_burn_shares(
        &self,
        org: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_burn_shares_and_watch(&signer, org, old_accounts)
            .await?
            .shares_batch_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn org_parent_child(
        &self,
        parent: <T as Org>::OrgId,
        child: <T as Org>::OrgId,
    ) -> bool {
        self.chain_client()
            .org_tree(parent, child, None)
            .await
            .is_ok()
    }
    async fn org(&self, org: <T as Org>::OrgId) -> Result<OrgState<T>> {
        Ok(self.chain_client().orgs(org, None).await?)
    }
    async fn share_profile(
        &self,
        org: <T as Org>::OrgId,
        account: <T as System>::AccountId,
    ) -> Result<Prof<T>> {
        Ok(self.chain_client().members(org, &account, None).await?)
    }
    async fn org_relations(&self) -> Result<Vec<Relacion<T>>> {
        let mut relations = self.chain_client().org_tree_iter(None).await?;
        let mut org_relations = Vec::<Relacion<T>>::new();
        while let Some((_, r)) = relations.next().await? {
            org_relations.push(r)
        }
        Ok(org_relations)
    }
    async fn org_members(
        &self,
        org: <T as Org>::OrgId,
    ) -> Result<Option<Vec<(T::AccountId, Prof<T>)>>> {
        let mut members = self.chain_client().members_iter(None).await?;
        let mut members_for_org = Vec::<(T::AccountId, Prof<T>)>::new();
        while let Some((_, profile)) = members.next().await? {
            if profile.id().0 == org {
                members_for_org.push((profile.id().1, profile));
            }
        }
        if members_for_org.is_empty() {
            Ok(None)
        } else {
            Ok(Some(members_for_org))
        }
    }
    async fn share_profiles(
        &self,
        account: <T as System>::AccountId,
    ) -> Result<Option<Vec<(T::OrgId, Prof<T>, OrgState<T>)>>> {
        let mut members = self.chain_client().members_iter(None).await?;
        let mut orgs_for_account =
            Vec::<(T::OrgId, Prof<T>, OrgState<T>)>::new();
        while let Some((_, profile)) = members.next().await? {
            if profile.id().1 == account {
                let org_state =
                    self.chain_client().orgs(profile.id().0, None).await?;
                orgs_for_account.push((profile.id().0, profile, org_state));
            }
        }
        if orgs_for_account.is_empty() {
            Ok(None)
        } else {
            Ok(Some(orgs_for_account))
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
        client::Client as _,
        mock::{
            test_node,
            AccountKeyring,
            Client,
        },
        org::{
            NewFlatOrganizationRegisteredEvent,
            OrgClient,
        },
        TextBlock,
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
    async fn new_flat_org_test() {
        let (node, _node_tmp) = test_node();
        let client = Client::mock(&node, AccountKeyring::Alice).await;
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
            .new_flat_org(
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
