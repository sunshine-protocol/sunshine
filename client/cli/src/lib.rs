pub mod bank;
pub mod bounty;
pub mod donate;
mod error;
pub mod key;
pub mod org;
pub mod shares;
pub mod vote;
pub mod wallet;

pub use crate::error::*;

use keystore::{
    bip39::{
        Language,
        Mnemonic,
    },
    DeviceKey,
    Password,
};
use substrate_subxt::system::System;

pub(crate) use async_trait::async_trait;
pub(crate) use bounty_client::{
    AbstractClient,
    Bank,
    Bounty,
    Donate,
    Org,
    Permill,
    Suri,
    Vote,
};
pub(crate) use substrate_subxt::{
    sp_core::Pair,
    Runtime,
};

#[async_trait]
pub trait Command<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair>:
    Send + Sync
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()>;
}

pub fn ask_for_new_password() -> Result<Password> {
    let password =
        ask_for_password("Please enter a new password (8+ characters):\n")?;
    let password2 = ask_for_password("Please confirm your new password:\n")?;
    if password != password2 {
        return Err(Error::PasswordMismatch)
    }
    Ok(password)
}

pub fn ask_for_password(prompt: &str) -> Result<Password> {
    Ok(Password::from(rpassword::prompt_password_stdout(prompt)?))
}

pub async fn ask_for_phrase(prompt: &str) -> Result<Mnemonic> {
    println!("{}", prompt);
    let mut words = Vec::with_capacity(24);
    while words.len() < 24 {
        let mut line = String::new();
        async_std::io::stdin().read_line(&mut line).await?;
        for word in line.split(' ') {
            words.push(word.trim().to_string());
        }
    }
    println!();
    Ok(Mnemonic::from_phrase(&words.join(" "), Language::English)
        .map_err(|_| Error::InvalidMnemonic)?)
}

pub async fn set_device_key<
    T: Runtime + Org + Vote + Donate + Bank + Bounty,
    P: Pair,
>(
    client: &dyn AbstractClient<T, P>,
    paperkey: bool,
    suri: Option<&str>,
    force: bool,
) -> Result<<T as System>::AccountId>
where
    P::Seed: Into<[u8; 32]> + Copy + Send + Sync,
{
    if client.has_device_key().await && !force {
        return Err(Error::HasDeviceKey)
    }
    let password = ask_for_new_password()?;
    if password.expose_secret().len() < 8 {
        return Err(Error::PasswordTooShort)
    }
    let dk = if paperkey {
        let mnemonic =
            ask_for_phrase("Please enter your backup phrase:").await?;
        DeviceKey::from_mnemonic(&mnemonic)
            .map_err(|_| Error::InvalidMnemonic)?
    } else if let Some(suri) = &suri {
        let suri: Suri<P> = suri.parse()?;
        DeviceKey::from_seed(suri.0.into())
    } else {
        DeviceKey::generate().await
    };
    Ok(client.set_device_key(&dk, &password, force).await?)
}
