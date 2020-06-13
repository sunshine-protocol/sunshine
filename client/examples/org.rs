use sp_keyring::AccountKeyring;
//#[cfg(feature = "light-client")]
//use sunshine_client::ChainType;
use ipfs_embed::{Config, Store};
use ipld_block_builder::{BlockBuilder, Codec};
use keystore::{DeviceKey, KeyStore, Password};
use sp_core::crypto::Pair;
use sunshine_client::{ClientBuilder, Error, Runtime, SunClient};
// use libipld::cid::{Cid, Codec};
// use libipld::multihash::Sha2_256;
// use utils_identity::cid::CidBytes;

#[async_std::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    let subxt = ClientBuilder::new().build().await.unwrap();
    let db = sled::open("tmp/db")?;
    let ipld_tree = db.open_tree("ipld_tree")?;
    let config = Config::from_tree(ipld_tree);
    let store = Store::new(config)?;
    let codec = Codec::new();
    let ipld = BlockBuilder::new(store, codec);
    let keystore = KeyStore::new("/tmp/keystore");
    // //#[cfg(not(feature = "light-client"))]
    let client = SunClient::new(subxt, keystore, ipld);
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
