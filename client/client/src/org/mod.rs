mod subxt;
mod utils;

pub use subxt::*;
pub use utils::AccountShare;

use crate::error::Error;
use libipld::{
    cache::Cache,
    cbor::DagCborCodec,
};
use substrate_subxt::{
    system::System,
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_client_utils::{
    async_trait,
    Client,
    Node,
    OffchainConfig,
    Result,
};

#[async_trait]
pub trait OrgClient<N: Node>: Client<N>
where
    N::Runtime: Org,
{
    async fn new_flat_org(
        &self,
        sudo: Option<<N::Runtime as System>::AccountId>,
        parent_org: Option<<N::Runtime as Org>::OrgId>,
        constitution: <N::Runtime as Org>::Constitution,
        members: &[<N::Runtime as System>::AccountId],
    ) -> Result<NewFlatOrgEvent<N::Runtime>>;
    async fn new_weighted_org(
        &self,
        sudo: Option<<N::Runtime as System>::AccountId>,
        parent_org: Option<<N::Runtime as Org>::OrgId>,
        constitution: <N::Runtime as Org>::Constitution,
        weighted_members: &[(
            <N::Runtime as System>::AccountId,
            <N::Runtime as Org>::Shares,
        )],
    ) -> Result<NewWeightedOrgEvent<N::Runtime>>;
    async fn issue_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        who: <N::Runtime as System>::AccountId,
        shares: <N::Runtime as Org>::Shares,
    ) -> Result<SharesIssuedEvent<N::Runtime>>;
    async fn burn_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        who: <N::Runtime as System>::AccountId,
        shares: <N::Runtime as Org>::Shares,
    ) -> Result<SharesBurnedEvent<N::Runtime>>;
    async fn batch_issue_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        new_accounts: &[(
            <N::Runtime as System>::AccountId,
            <N::Runtime as Org>::Shares,
        )],
    ) -> Result<SharesBatchIssuedEvent<N::Runtime>>;
    async fn batch_burn_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        old_accounts: &[(
            <N::Runtime as System>::AccountId,
            <N::Runtime as Org>::Shares,
        )],
    ) -> Result<SharesBatchBurnedEvent<N::Runtime>>;
    async fn org_parent_child(
        &self,
        parent: <N::Runtime as Org>::OrgId,
        child: <N::Runtime as Org>::OrgId,
    ) -> bool;
    async fn org(
        &self,
        org: <N::Runtime as Org>::OrgId,
    ) -> Result<OrgState<N::Runtime>>;
    async fn share_profile(
        &self,
        org: <N::Runtime as Org>::OrgId,
        account: <N::Runtime as System>::AccountId,
    ) -> Result<Prof<N::Runtime>>;
    async fn org_relations(&self) -> Result<Vec<Relacion<N::Runtime>>>;
    async fn org_members(
        &self,
        org: <N::Runtime as Org>::OrgId,
    ) -> Result<
        Option<Vec<(<N::Runtime as System>::AccountId, Prof<N::Runtime>)>>,
    >;
    async fn share_profiles(
        &self,
        account: <N::Runtime as System>::AccountId,
    ) -> Result<
        Option<
            Vec<(
                <N::Runtime as Org>::OrgId,
                Prof<N::Runtime>,
                OrgState<N::Runtime>,
            )>,
        >,
    >;
}

#[async_trait]
impl<N, C> OrgClient<N> for C
where
    N: Node,
    N::Runtime: Org,
    <<<N::Runtime as Runtime>::Extra as SignedExtra<N::Runtime>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <N::Runtime as Org>::Cid: From<libipld::cid::Cid>,
    C: Client<N>,
    C::OffchainClient: libipld::cache::Cache<
        OffchainConfig<N>,
        DagCborCodec,
        <N::Runtime as Org>::Constitution,
    >,
{
    async fn new_flat_org(
        &self,
        sudo: Option<<N::Runtime as System>::AccountId>,
        parent_org: Option<<N::Runtime as Org>::OrgId>,
        constitution: <N::Runtime as Org>::Constitution,
        members: &[<N::Runtime as System>::AccountId],
    ) -> Result<NewFlatOrgEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        let constitution = self.offchain_client().insert(constitution).await?;
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
        sudo: Option<<N::Runtime as System>::AccountId>,
        parent_org: Option<<N::Runtime as Org>::OrgId>,
        constitution: <N::Runtime as Org>::Constitution,
        weighted_members: &[(<N::Runtime as System>::AccountId, <N::Runtime as Org>::Shares)],
    ) -> Result<NewWeightedOrgEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        let constitution = self.offchain_client().insert(constitution).await?;
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
        org: <N::Runtime as Org>::OrgId,
        who: <N::Runtime as System>::AccountId,
        shares: <N::Runtime as Org>::Shares,
    ) -> Result<SharesIssuedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .issue_shares_and_watch(&signer, org, &who, shares)
            .await?
            .shares_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn burn_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        who: <N::Runtime as System>::AccountId,
        shares: <N::Runtime as Org>::Shares,
    ) -> Result<SharesBurnedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .burn_shares_and_watch(&signer, org, &who, shares)
            .await?
            .shares_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_issue_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        new_accounts: &[(<N::Runtime as System>::AccountId, <N::Runtime as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_issue_shares_and_watch(&signer, org, new_accounts)
            .await?
            .shares_batch_issued()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn batch_burn_shares(
        &self,
        org: <N::Runtime as Org>::OrgId,
        old_accounts: &[(<N::Runtime as System>::AccountId, <N::Runtime as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .batch_burn_shares_and_watch(&signer, org, old_accounts)
            .await?
            .shares_batch_burned()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn org_parent_child(
        &self,
        parent: <N::Runtime as Org>::OrgId,
        child: <N::Runtime as Org>::OrgId,
    ) -> bool {
        self.chain_client()
            .org_tree(parent, child, None)
            .await
            .is_ok()
    }
    async fn org(&self, org: <N::Runtime as Org>::OrgId) -> Result<OrgState<N::Runtime>> {
        Ok(self.chain_client().orgs(org, None).await?)
    }
    async fn share_profile(
        &self,
        org: <N::Runtime as Org>::OrgId,
        account: <N::Runtime as System>::AccountId,
    ) -> Result<Prof<N::Runtime>> {
        Ok(self.chain_client().members(org, &account, None).await?)
    }
    async fn org_relations(&self) -> Result<Vec<Relacion<N::Runtime>>> {
        let mut relations = self.chain_client().org_tree_iter(None).await?;
        let mut org_relations = Vec::<Relacion<N::Runtime>>::new();
        while let Some((_, r)) = relations.next().await? {
            org_relations.push(r)
        }
        Ok(org_relations)
    }
    async fn org_members(
        &self,
        org: <N::Runtime as Org>::OrgId,
    ) -> Result<Option<Vec<(<N::Runtime as System>::AccountId, Prof<N::Runtime>)>>> {
        let mut members = self.chain_client().members_iter(None).await?;
        let mut members_for_org = Vec::new();
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
        account: <N::Runtime as System>::AccountId,
    ) -> Result<Option<Vec<(<N::Runtime as Org>::OrgId, Prof<N::Runtime>, OrgState<N::Runtime>)>>> {
        let mut members = self.chain_client().members_iter(None).await?;
        let mut orgs_for_account = Vec::new();
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
        client::{
            AccountKeyring,
            Client as _,
            Node as _,
        },
        org::{
            NewFlatOrgEvent,
            OrgClient,
        },
        Client,
        Node,
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
        let node = Node::new_mock();
        let (client, _tmp) = Client::mock(&node, AccountKeyring::Alice).await;
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
        let node = Node::new_mock();
        let (client, _tmp) = Client::mock(&node, AccountKeyring::Alice).await;
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
        let expected_event = NewFlatOrgEvent {
            caller: alice_account_id,
            new_id: 2,
            constitution: event.constitution.clone(),
            total: 7,
        };
        assert_eq!(event, expected_event);
    }
}
