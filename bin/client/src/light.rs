use crate::Runtime;
use sled::{
    transaction::TransactionError,
    Tree,
};
use sp_database::{
    Change,
    Database,
    Transaction,
};
use std::sync::Arc;
use substrate_subxt::{
    client::{
        DatabaseConfig,
        Role,
        SubxtClient,
        SubxtClientConfig,
    },
    Client,
    ClientBuilder,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error(transparent)]
    ScService(#[from] sc_service::Error),
    #[error("Invalid chain spec: {0}")]
    ChainSpec(#[from] ChainSpecError),
}

#[derive(Debug, Error)]
#[error("Invalid chain spec: {0}")]
pub struct ChainSpecError(String);

pub async fn build_light_client(
    tree: Tree,
    chain_spec: &'static [u8],
) -> Result<Client<Runtime>, Error> {
    let chain_spec =
        test_node::chain_spec::ChainSpec::from_json_bytes(chain_spec)
            .map_err(ChainSpecError)?;
    let config = SubxtClientConfig {
        impl_name: test_node::IMPL_NAME,
        impl_version: test_node::IMPL_VERSION,
        author: test_node::AUTHOR,
        copyright_start_year: test_node::COPYRIGHT_START_YEAR,
        db: DatabaseConfig::Custom(Arc::new(SubstrateDb(tree))),
        builder: test_node::service::new_light,
        role: Role::Light,
        chain_spec,
    };
    let client = ClientBuilder::new()
        .set_client(SubxtClient::new(config)?)
        .build()
        .await?;
    Ok(client)
}

struct Key;

impl Key {
    pub fn key(col: u32, key: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + 4 + key.len());
        buf.push(0);
        buf.extend_from_slice(&col.to_be_bytes());
        buf.extend_from_slice(key);
        buf
    }

    pub fn hash_key(hash: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1 + hash.len());
        buf.push(1);
        buf.extend_from_slice(hash);
        buf
    }
}

struct SubstrateDb(Tree);

impl<H> Database<H> for SubstrateDb
where
    H: Clone + Send + Sync + Eq + PartialEq + Default + AsRef<[u8]>,
{
    fn commit(&self, transaction: Transaction<H>) {
        let changes = &transaction.0;
        self.0
            .transaction::<_, _, TransactionError>(|tree| {
                for change in changes.into_iter() {
                    match change {
                        Change::Set(col, key, value) => {
                            tree.insert(Key::key(*col, key), value.as_slice())?;
                        }
                        Change::Remove(col, key) => {
                            tree.remove(Key::key(*col, key))?;
                        }
                        Change::Store(hash, preimage) => {
                            tree.insert(
                                Key::hash_key(hash.as_ref()),
                                preimage.as_slice(),
                            )?;
                        }
                        Change::Release(hash) => {
                            tree.remove(Key::hash_key(hash.as_ref()))?;
                        }
                    }
                }
                Ok(())
            })
            .ok();
    }

    fn get(&self, col: u32, key: &[u8]) -> Option<Vec<u8>> {
        self.0
            .get(Key::key(col, key))
            .ok()
            .unwrap_or_default()
            .map(|ivec| ivec.to_vec())
    }

    fn lookup(&self, hash: &H) -> Option<Vec<u8>> {
        self.0
            .get(Key::hash_key(hash.as_ref()))
            .ok()
            .unwrap_or_default()
            .map(|ivec| ivec.to_vec())
    }
}
