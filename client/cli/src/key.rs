use crate::{
    ask_for_password,
    async_trait,
    set_device_key,
    AbstractClient,
    Command,
    Org,
    Pair,
    Result,
    Runtime,
    Vote,
};
use clap::Clap;

#[derive(Clone, Debug, Clap)]
pub struct KeySetCommand {
    /// Overwrite existing keys.
    #[clap(short = "f", long = "force")]
    pub force: bool,

    /// Suri.
    #[clap(long = "suri")]
    pub suri: Option<String>,

    /// Paperkey.
    #[clap(long = "paperkey")]
    pub paperkey: bool,
}

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P> for KeySetCommand
where
    P::Seed: Into<[u8; 32]> + Copy + Send + Sync,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account_id = set_device_key(
            client,
            self.paperkey,
            self.suri.as_deref(),
            self.force,
        )
        .await?;
        let account_id_str = account_id.to_string();
        println!("Your device id is {}", &account_id_str);
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct KeyLockCommand;

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P> for KeyLockCommand {
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        client.lock().await?;
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct KeyUnlockCommand;

#[async_trait]
impl<T: Runtime + Org + Vote, P: Pair> Command<T, P> for KeyUnlockCommand {
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let password =
            ask_for_password("Please enter your password (8+ characters):\n")?;
        client.unlock(&password).await?;
        Ok(())
    }
}
