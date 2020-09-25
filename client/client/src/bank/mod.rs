mod subxt;

pub use subxt::*;

use crate::{
    error::Error,
    org::Org,
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
    Result,
};

#[async_trait]
pub trait BankClient<N: Node>: Client<N>
where
    N::Runtime: Bank,
{
    async fn open(
        &self,
        seed: BalanceOf<N::Runtime>,
        hosting_org: <N::Runtime as Org>::OrgId,
        bank_operator: Option<<N::Runtime as System>::AccountId>,
        threshold: Threshold<N::Runtime>,
    ) -> Result<AccountOpenedEvent<N::Runtime>>;
    async fn propose_spend(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        amount: BalanceOf<N::Runtime>,
        dest: <N::Runtime as System>::AccountId,
    ) -> Result<SpendProposedEvent<N::Runtime>>;
    async fn trigger_vote(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<VoteTriggeredEvent<N::Runtime>>;
    async fn sudo_approve(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<SudoApprovedEvent<N::Runtime>>;
    async fn close(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
    ) -> Result<AccountClosedEvent<N::Runtime>>;
    async fn bank(&self, bank_id: <N::Runtime as Bank>::BankId) -> Result<BankSt<N::Runtime>>;
    async fn spend_proposal(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<SpendProp<N::Runtime>>;
    async fn banks_for_org(
        &self,
        org: <N::Runtime as Org>::OrgId,
    ) -> Result<Option<Vec<(<N::Runtime as Bank>::BankId, BankSt<N::Runtime>)>>>;
}

#[async_trait]
impl<N, C> BankClient<N> for C
where
    N: Node,
    N::Runtime: Bank,
    <<<N::Runtime as Runtime>::Extra as SignedExtra<N::Runtime>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<N>,
{
    async fn open(
        &self,
        seed: BalanceOf<N::Runtime>,
        hosting_org: <N::Runtime as Org>::OrgId,
        bank_operator: Option<<N::Runtime as System>::AccountId>,
        threshold: Threshold<N::Runtime>,
    ) -> Result<AccountOpenedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .open_and_watch(
                &signer,
                seed,
                hosting_org,
                bank_operator,
                threshold,
            )
            .await?
            .account_opened()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn propose_spend(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        amount: BalanceOf<N::Runtime>,
        dest: <N::Runtime as System>::AccountId,
    ) -> Result<SpendProposedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .propose_spend_and_watch(&signer, bank_id, amount, dest)
            .await?
            .spend_proposed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn trigger_vote(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<VoteTriggeredEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .trigger_vote_and_watch(&signer, bank_id, spend_id)
            .await?
            .vote_triggered()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn sudo_approve(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<SudoApprovedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .sudo_approve_and_watch(&signer, bank_id, spend_id)
            .await?
            .sudo_approved()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn close(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
    ) -> Result<AccountClosedEvent<N::Runtime>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .close_and_watch(&signer, bank_id)
            .await?
            .account_closed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn bank(&self, bank_id: <N::Runtime as Bank>::BankId) -> Result<BankSt<N::Runtime>> {
        Ok(self.chain_client().banks(bank_id, None).await?)
    }
    async fn spend_proposal(
        &self,
        bank_id: <N::Runtime as Bank>::BankId,
        spend_id: <N::Runtime as Bank>::SpendId,
    ) -> Result<SpendProp<N::Runtime>> {
        Ok(self
            .chain_client()
            .spend_proposals(bank_id, spend_id, None)
            .await?)
    }
    async fn banks_for_org(
        &self,
        org: <N::Runtime as Org>::OrgId,
    ) -> Result<Option<Vec<(<N::Runtime as Bank>::BankId, BankSt<N::Runtime>)>>> {
        let mut banks = self.chain_client().banks_iter(None).await?;
        let mut banks_for_org = Vec::new();
        while let Some((_, bank)) = banks.next().await? {
            if bank.org() == org {
                banks_for_org.push((bank.id(), bank));
            }
        }
        if banks_for_org.is_empty() {
            Ok(None)
        } else {
            Ok(Some(banks_for_org))
        }
    }
}
