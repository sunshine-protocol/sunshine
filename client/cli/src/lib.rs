mod error;

pub use crate::error::*;

use keystore::{
    DeviceKey,
    Password,
};
use substrate_subxt::system::System;

pub(crate) use async_trait::async_trait;
pub(crate) use bounty_client::{
    AbstractClient,
    Org,
};
pub(crate) use substrate_subxt::{
    sp_core::Pair,
    Runtime,
};

#[async_trait]
pub trait Command<T: Runtime + Org, P: Pair>: Send + Sync {
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()>;
}

pub async fn set_device_key<T: Runtime + Org, P: Pair>(
    client: &dyn AbstractClient<T, P>,
    dk: &DeviceKey,
    password: &Password,
    force: bool,
) -> Result<<T as System>::AccountId>
where
    P::Seed: Into<[u8; 32]> + Copy + Send + Sync,
{
    if client.has_device_key().await && !force {
        return Err(Error::HasDeviceKey)
    }
    Ok(client.set_device_key(&dk, &password, force).await?)
}
