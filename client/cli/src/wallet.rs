use crate::{
    async_trait,
    AbstractClient,
    Bank,
    Bounty,
    Command,
    Donate,
    Error,
    Org,
    Pair,
    Result,
    Runtime,
    Vote,
};
use bounty_client::Account;
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    balances::{
        Balances,
        TransferCallExt,
        TransferEventExt,
    },
    sp_core::crypto::Ss58Codec,
    system::{
        AccountStoreExt,
        System,
    },
    SignedExtension,
    SignedExtra,
};

#[derive(Clone, Debug, Clap)]
pub struct WalletBalanceCommand {
    pub account: String,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty + Balances, P: Pair>
    Command<T, P> for WalletBalanceCommand
where
    <T as System>::AccountId: Ss58Codec,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.account.parse()?;
        let amount = client.subxt().account(&account.id, None).await?;
        println!(
            "AccountId {:?} has account balance data {:?}",
            account.id, amount.data
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct WalletTransferCommand {
    pub dest: String,
    pub amount: u128,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty + Balances, P: Pair> Command<T, P> for WalletTransferCommand
where
    <T as System>::AccountId: Ss58Codec + Into<<T as System>::Address>,
    <<<T as Runtime>::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    <T as Balances>::Balance: From<u128> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account: Account<T> = self.dest.parse()?;
        let signer = client.signer().await?;
        let event = client
            .subxt()
            .transfer_and_watch(&*signer, &account.id.into(), self.amount.into())
            .await?
            .transfer()
            .map_err(|_| Error::TransferEventDecode)?
            .ok_or(Error::TransferEventFind)?;
        println!("transferred {} to {}", event.amount, event.to.to_string());
        Ok(())
    }
}
