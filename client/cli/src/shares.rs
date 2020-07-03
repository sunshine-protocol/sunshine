use crate::{
    async_trait,
    AbstractClient,
    Command,
    Org,
    Pair,
    Result,
    Runtime,
};
use bounty_client::{
    Account,
    AccountShare,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
};

#[derive(Clone, Debug, Clap)]
pub struct SharesIssueCommand {
    pub organization: u64,
    pub dest: String,
    pub shares: u64,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesIssueCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.dest.parse()?;
        let event = client
            .issue_shares(
                self.organization.into(),
                account.id,
                self.shares.into(),
            )
            .await?;
        println!(
            "{} shares minted for account {:?} in the context of Org {}",
            event.shares, event.who, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesBatchIssueCommand {
    pub organization: u64,
    pub new_accounts: Vec<AccountShare>,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesBatchIssueCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let accounts = self.new_accounts.iter().map(|acc_share| -> Result<(<T as System>::AccountId, <T as Org>::Shares)> {
            let account: Account<T> = acc_share.0.parse()?;
            let amount_issued: T::Shares = (acc_share.1).into();
            Ok((account.id, amount_issued))
        }).collect::<Result<Vec<(<T as System>::AccountId, <T as Org>::Shares)>>>()?;
        let event = client
            .batch_issue_shares(self.organization.into(), accounts.as_slice())
            .await?;
        println!(
            "{} new shares minted in the context of Org {}",
            event.total_new_shares_minted, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesBatchBurnCommand {
    pub organization: u64,
    pub old_accounts: Vec<AccountShare>,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesBatchBurnCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let accounts = self.old_accounts.iter().map(|acc_share| -> Result<(<T as System>::AccountId, <T as Org>::Shares)> {
            let account: Account<T> = acc_share.0.parse()?;
            let amount_burned: T::Shares = (acc_share.1).into();
            Ok((account.id, amount_burned))
        }).collect::<Result<Vec<(<T as System>::AccountId, <T as Org>::Shares)>>>()?;
        let event = client
            .batch_issue_shares(self.organization.into(), accounts.as_slice())
            .await?;
        println!(
            "{} new shares minted in the context of Org {}",
            event.total_new_shares_minted, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesBurnCommand {
    pub organization: u64,
    pub burner: String,
    pub shares: u64,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesBurnCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.burner.parse()?;
        let event = client
            .issue_shares(
                self.organization.into(),
                account.id,
                self.shares.into(),
            )
            .await?;
        println!(
            "{} shares burned from account {:?} in the context of Org {}",
            event.shares, event.who, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesReserveCommand {
    pub organization: u64,
    pub who: String,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesReserveCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.who.parse()?;
        let event = client
            .reserve_shares(self.organization.into(), &account.id)
            .await?;
        println!(
            "Account {} reserves {:?} shares in the context of Org {}",
            event.who, event.amount_reserved, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesUnReserveCommand {
    pub organization: u64,
    pub who: String,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesUnReserveCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.who.parse()?;
        let event = client
            .unreserve_shares(self.organization.into(), &account.id)
            .await?;
        println!(
            "Account {} unreserves {:?} shares in the context of Org {}",
            event.who, event.amount_unreserved, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesLockCommand {
    pub organization: u64,
    pub who: String,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesLockCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.who.parse()?;
        let event = client
            .lock_shares(self.organization.into(), &account.id)
            .await?;
        println!(
            "Locked shares for Account {} in the context of Org {}",
            event.who, event.organization
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SharesUnLockCommand {
    pub organization: u64,
    pub who: String,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for SharesUnLockCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.who.parse()?;
        let event = client
            .unlock_shares(self.organization.into(), &account.id)
            .await?;
        println!(
            "Unlocked shares for Account {} in the context of Org {}",
            event.who, event.organization
        );
        Ok(())
    }
}
