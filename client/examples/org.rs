use sp_keyring::AccountKeyring;
//#[cfg(feature = "light-client")]
//use sunshine_client::ChainType;
use ipfs_embed::{Config, Store};
use ipld_block_builder::{BlockBuilder, Codec};
use keystore::{DeviceKey, KeyStore, Password};
use sp_core::sr25519::Pair;
use substrate_subxt::ClientBuilder;
use sunshine_client::{Error, Runtime, SunClient};
// use libipld::cid::{Cid, Codec};
// use libipld::multihash::Sha2_256;
// use utils_identity::cid::CidBytes;

#[async_std::main]
async fn main() -> Result<(), Error> {
    env_logger::init();
    //let subxt = ClientBuilder::<Runtime>::new().build().await?;
    let db = sled::open("/tmp/db")?;
    let ipld_tree = db.open_tree("ipld_tree")?;
    let config = Config::from_tree(ipld_tree);
    let store = Store::new(config)?;
    let codec = Codec::new();
    let ipld = BlockBuilder::new(store, codec);
    let keystore = KeyStore::new("/tmp/keystore");
    let alice_seed: [u8; 32] = AccountKeyring::Alice.into();
    let _ = keystore.initialize(
        &DeviceKey::from_seed(alice_seed),
        &Password::from("password".to_string()),
    )?;
    //let client = SunClient::<_, _, _, Pair, _>::new(keystore, subxt, ipld);
    let account_id = sp_keyring::AccountKeyring::Alice.to_account_id();
    println!("{}", account_id);
    //client.issue_shares(1u64, account_id, 10u64).await?;

    // println!(
    //     "Account {:?} was issued {:?} shares for organization {:?}",
    //     event.who, event.shares, event.organization,
    // );

    Ok(())
}
