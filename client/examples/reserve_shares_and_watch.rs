use sp_keyring::AccountKeyring;
#[cfg(feature = "light-client")]
use sunshine_client::ChainType;
use sunshine_client::{Error, Sunshine};

#[async_std::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let signer = AccountKeyring::Alice.pair();
    #[cfg(not(feature = "light-client"))]
    let client = Sunshine::new("/tmp/db", signer).await?;
    #[cfg(feature = "light-client")]
    let client = Sunshine::new("/tmp/db", signer, ChainType::Development).await?;

    let event = client.reserve_shares(1, 1).await?;

    println!(
        "Account {:?} reserved {:?} shares with share id {:?} for organization {:?}",
        event.account, event.reserved, event.share, event.org,
    );

    Ok(())
}
