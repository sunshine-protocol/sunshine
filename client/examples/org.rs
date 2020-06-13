use sp_keyring::AccountKeyring;
//#[cfg(feature = "light-client")]
//use sunshine_client::ChainType;
use keystore::{DeviceKey, KeyStore, Password};
use sp_core::crypto::Pair;
use sunshine_client::{Error, SunClient};
// use libipld::cid::{Cid, Codec};
// use libipld::multihash::Sha2_256;
// use substrate_subxt::ClientBuilder;
// use utils_identity::cid::CidBytes;

#[async_std::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    // //#[cfg(not(feature = "light-client"))]
    let client = SunClient::new("/tmp/db", KeyStore::new("/tmp/keystore")).await?;
    // #[cfg(feature = "light-client")]
    // let client = Sunshine::new("/tmp/db", signer, ChainType::Development).await?;
    let account_id = sp_keyring::AccountKeyring::Alice.to_account_id();
    let event = client.reserve_shares(1u64, &account_id).await?;

    println!(
        "Account {:?} reserved {:?} shares for organization {:?}",
        event.who, event.amount, event.org,
    );

    Ok(())
}
