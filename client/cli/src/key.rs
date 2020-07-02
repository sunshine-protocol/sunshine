use crate::{
    ask_for_password, async_trait, set_device_key, AbstractClient, Command, Org, Pair, Result,
    Runtime,
};
use clap::Clap;
use textwrap::Wrapper;

#[derive(Clone, Debug, Clap)]
pub struct KeySetCommand {
    /// Overwrite existing keys.
    #[clap(short = "f", long = "force")]
    pub force: bool,

    /// Suri.
    #[clap(long = "suri")]
    pub suri: Option<Suri>,

    /// Paperkey.
    #[clap(long = "paperkey")]
    pub paperkey: bool,
}

#[async_trait]
impl<T: Runtime + Org, P: Pair> Command<T, P> for KeySetCommand
where
    P::Seed: Into<[u8; 32]> + Copy + Send + Sync,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let account_id =
            set_device_key(client, self.paperkey, self.suri.as_deref(), self.force).await?;
        let account_id_str = account_id.to_string();
        println!("Your device id is {}", &account_id_str);
        Ok(())
    }
}