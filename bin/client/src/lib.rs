use bounty_client::{
    bank::Bank,
    bounty::Bounty,
    donate::Donate,
    org::Org,
    vote::Vote,
};
use identity_utils::cid::CidBytes;
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
    },
    sp_core,
    sp_runtime,
    sp_runtime::traits::{
        IdentifyAccount,
        Verify,
    },
    system::System,
};
use sunshine_core::{ChainClient, ChainSigner, Keystore as _, OffchainSigner};
use substrate_keybase_keystore::Keystore;
use std::path::Path;
use thiserror::Error;

pub use bounty_client as bounty;

type AccountId = <<sp_runtime::MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Runtime;

impl System for Runtime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type AccountId = AccountId;
    type Address = AccountId;
    type Header = sp_runtime::generic::Header<Self::BlockNumber, Self::Hashing>;
    type Extrinsic = sp_runtime::OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for Runtime {
    type Balance = u128;
}

impl Org for Runtime {
    type IpfsReference = CidBytes;
    type OrgId = u64;
    type Shares = u64;
}

impl Vote for Runtime {
    type VoteId = u64;
    type Signal = u64;
}

impl Donate for Runtime {
    type DCurrency = u128;
}

impl Bank for Runtime {
    type SpendId = u64;
    type Currency = u128;
}

impl Bounty for Runtime {
    type BountyId = u64;
}

impl substrate_subxt::Runtime for Runtime {
    type Signature = sp_runtime::MultiSignature;
    type Extra = substrate_subxt::DefaultExtra<Self>;
}

pub struct Client {
    keystore: Keystore<Runtime, sp_core::sr25519::Pair>,
    chain: substrate_subxt::Client<Runtime>,
}

impl Client {
    pub async fn new(root: &Path, _chain_spec: Option<&Path>) -> Result<Self, Error> {
        let keystore = Keystore::open(root.join("keystore")).await?;
        let chain = substrate_subxt::ClientBuilder::new().build().await?;
        Ok(Self {
            keystore,
            chain,
        })
    }

    #[cfg(feature = "mock")]
    pub async fn mock(
        test_node: &mock::TestNode,
        account: sp_keyring::AccountKeyring,
    ) -> (Self, tempdir::TempDir) {
        use substrate_keybase_keystore::Key;
        use substrate_subxt::ClientBuilder;
        use sunshine_core::{Key as _, SecretString};
        use tempdir::TempDir;

        let tmp = TempDir::new("sunshine-identity-").expect("failed to create tempdir");
        let chain = ClientBuilder::new()
            .set_client(test_node.clone())
            .build()
            .await
            .unwrap();
        let mut keystore = Keystore::open(tmp.path().join("keystore")).await.unwrap();
        let key = Key::from_suri(&account.to_seed()).unwrap();
        let password = SecretString::new("password".to_string());
        keystore
            .set_device_key(&key, &password, false)
            .await
            .unwrap();
        (
            Self {
                keystore,
                chain,
            },
            tmp,
        )
    }
}

impl ChainClient<Runtime> for Client {
    type Keystore = Keystore<Runtime, sp_core::sr25519::Pair>;
    type OffchainClient = ();
    type Error = Error;

    fn keystore(&self) -> &Self::Keystore {
        &self.keystore
    }

    fn keystore_mut(&mut self) -> &mut Self::Keystore {
        &mut self.keystore
    }

    fn chain_client(&self) -> &substrate_subxt::Client<Runtime> {
        &self.chain
    }

    fn chain_signer(&self) -> Result<&(dyn ChainSigner<Runtime> + Send + Sync), Self::Error> {
        self.keystore
            .chain_signer()
            .ok_or(Error::Keystore(substrate_keybase_keystore::Error::Locked))
    }

    fn offchain_client(&self) -> &Self::OffchainClient {
        &()
    }

    fn offchain_signer(&self) -> Result<&dyn OffchainSigner<Runtime>, Self::Error> {
        self.keystore
            .offchain_signer()
            .ok_or(Error::Keystore(substrate_keybase_keystore::Error::Locked))
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Keystore(#[from] substrate_keybase_keystore::Error),
    #[error(transparent)]
    Chain(#[from] substrate_subxt::Error),
    #[error(transparent)]
    Ipld(#[from] libipld::error::Error),
    #[error(transparent)]
    Bounty(#[from] bounty_client::Error),
}

impl From<codec::Error> for Error {
    fn from(error: codec::Error) -> Self {
        Self::Chain(error.into())
    }
}

#[cfg(feature = "mock")]
pub mod mock {
    pub use sp_keyring::AccountKeyring;
    use substrate_subxt::client::{
        DatabaseConfig,
        Role,
        SubxtClient,
        SubxtClientConfig,
    };
    pub use tempdir::TempDir;

    pub type TestNode = jsonrpsee::Client;

    pub fn test_node() -> (TestNode, TempDir) {
        env_logger::try_init().ok();
        let tmp =
            TempDir::new("sunshine-bounty-node").expect("failed to create tempdir");
        let config = SubxtClientConfig {
            impl_name: test_node::IMPL_NAME,
            impl_version: test_node::IMPL_VERSION,
            author: test_node::AUTHOR,
            copyright_start_year: test_node::COPYRIGHT_START_YEAR,
            db: DatabaseConfig::RocksDb {
                path: tmp.path().into(),
                cache_size: 128,
            },
            builder: test_node::service::new_full,
            chain_spec: test_node::chain_spec::development_config(),
            role: Role::Authority(AccountKeyring::Alice),
        };
        let client = SubxtClient::new(config).unwrap().into();
        (client, tmp)
    }
}
