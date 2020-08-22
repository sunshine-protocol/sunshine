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
    Result,
};

#[async_trait]
pub trait BankClient<T: Runtime + Bank>: Client<T> {
    async fn open(
        &self,
        seed: BalanceOf<T>,
        hosting_org: <T as Org>::OrgId,
        bank_operator: Option<<T as System>::AccountId>,
    ) -> Result<AccountOpenedEvent<T>>;
    async fn propose_spend(
        &self,
        bank_id: <T as Bank>::BankId,
        amount: BalanceOf<T>,
        dest: <T as System>::AccountId,
    ) -> Result<SpendProposedEvent<T>>;
    async fn trigger_vote(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<VoteTriggeredEvent<T>>;
    async fn sudo_approve(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<SudoApprovedEvent<T>>;
    async fn close(
        &self,
        bank_id: <T as Bank>::BankId,
    ) -> Result<AccountClosedEvent<T>>;
    async fn bank(&self, bank_id: <T as Bank>::BankId) -> Result<BankSt<T>>;
    async fn spend_proposal(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<SpendProp<T>>;
    async fn banks_for_org(
        &self,
        org: <T as Org>::OrgId,
    ) -> Result<Option<Vec<(T::BankId, BankSt<T>)>>>;
}

#[async_trait]
impl<T, C> BankClient<T> for C
where
    T: Runtime + Bank,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<T>,
{
    async fn open(
        &self,
        seed: BalanceOf<T>,
        hosting_org: <T as Org>::OrgId,
        bank_operator: Option<<T as System>::AccountId>,
    ) -> Result<AccountOpenedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .open_and_watch(&signer, seed, hosting_org, bank_operator)
            .await?
            .account_opened()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn propose_spend(
        &self,
        bank_id: <T as Bank>::BankId,
        amount: BalanceOf<T>,
        dest: <T as System>::AccountId,
    ) -> Result<SpendProposedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .propose_spend_and_watch(&signer, bank_id, amount, dest)
            .await?
            .spend_proposed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn trigger_vote(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<VoteTriggeredEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .trigger_vote_and_watch(&signer, bank_id, spend_id)
            .await?
            .vote_triggered()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn sudo_approve(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<SudoApprovedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .sudo_approve_and_watch(&signer, bank_id, spend_id)
            .await?
            .sudo_approved()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn close(
        &self,
        bank_id: <T as Bank>::BankId,
    ) -> Result<AccountClosedEvent<T>> {
        let signer = self.chain_signer()?;
        self.chain_client()
            .close_and_watch(&signer, bank_id)
            .await?
            .account_closed()?
            .ok_or_else(|| Error::EventNotFound.into())
    }
    async fn bank(&self, bank_id: <T as Bank>::BankId) -> Result<BankSt<T>> {
        Ok(self.chain_client().banks(bank_id, None).await?)
    }
    async fn spend_proposal(
        &self,
        bank_id: <T as Bank>::BankId,
        spend_id: <T as Bank>::SpendId,
    ) -> Result<SpendProp<T>> {
        Ok(self
            .chain_client()
            .spend_proposals(bank_id, spend_id, None)
            .await?)
    }
    async fn banks_for_org(
        &self,
        org: <T as Org>::OrgId,
    ) -> Result<Option<Vec<(T::BankId, BankSt<T>)>>> {
        let mut banks = self.chain_client().banks_iter(None).await?;
        let mut banks_for_org = Vec::<(T::BankId, BankSt<T>)>::new();
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
