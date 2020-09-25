use libipld::{
    cache::{
        IpldCache,
    },
    cbor::DagCborCodec,
    derive_cache,
    store::Store,
};
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
    },
    extrinsic,
    sp_core,
    sp_runtime,
    sp_runtime::traits::{
        IdentifyAccount,
        Verify,
    },
    system::System,
};
use sunshine_bounty_client::{
    bank::Bank,
    bounty::Bounty,
    donate::Donate,
    org::Org,
    vote::Vote,
};
use sunshine_client_utils::{
    GenericClient,
    OffchainStore,
    ChainSpecError,
    Node as NodeT,
    Network,
    sc_service::{
        self,
        Configuration,
        RpcHandlers,
        TaskManager,
    },
    codec::hasher::BLAKE2B_256,
    crypto::{
        keychain::KeyType,
        sr25519,
    },
};
use std::ops::Deref;

pub use sunshine_bounty_client::*;
pub use sunshine_bounty_utils as utils;
pub use sunshine_client_utils as client;

pub type AccountId = <<sp_runtime::MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type Uid = u32;

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
    type AccountData = AccountData<u128>;
}

impl Balances for Runtime {
    type Balance = u128;
}

impl Org for Runtime {
    type Cid = sunshine_codec::Cid;
    type OrgId = u64;
    type Shares = u64;
    type Constitution = TextBlock;
}

impl Vote for Runtime {
    type VoteId = u64;
    type Signal = u64;
    type ThresholdId = u64;
    type Percent = sp_runtime::Permill;
    type VoteTopic = TextBlock;
    type VoterView = utils::vote::VoterView;
    type VoteJustification = TextBlock;
}

impl Donate for Runtime {}

impl Bank for Runtime {
    type BankId = u64;
    type SpendId = u64;
}

impl Bounty for Runtime {
    type IpfsReference = sunshine_codec::Cid;
    type BountyId = u64;
    type BountyPost = GithubIssue;
    type SubmissionId = u64;
    type BountySubmission = GithubIssue;
}

impl sunshine_identity_client::Identity for Runtime {
    type Uid = u8;
    type Cid = sunshine_codec::Cid;
    type Mask = u8;
    type Gen = u16;
    type IdAccountData = ();
}

impl sunshine_faucet_client::Faucet for Runtime {}

impl substrate_subxt::Runtime for Runtime {
    type Signature = sp_runtime::MultiSignature;
    type Extra = extrinsic::DefaultExtra<Self>;
}

pub struct OffchainClient<S> {
    store: S,
    bounties: IpldCache<S, DagCborCodec, GithubIssue>,
    constitutions: IpldCache<S, DagCborCodec, TextBlock>,
}

impl<S: Store> OffchainClient<S> {
    pub fn new(store: S) -> Self {
        Self {
            bounties: IpldCache::new(store.clone(), DagCborCodec, BLAKE2B_256, 64),
            constitutions: IpldCache::new(store.clone(), DagCborCodec, BLAKE2B_256, 64),
            store,
        }
    }
}

derive_cache!(OffchainClient, bounties, DagCborCodec, GithubIssue);
derive_cache!(OffchainClient, constitutions, DagCborCodec, TextBlock);

impl<S: Store> From<S> for OffchainClient<S> {
    fn from(store: S) -> Self {
        Self::new(store)
    }
}

impl<S: Store> Deref for OffchainClient<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.store
    }
}

impl<S: Store> sunshine_client_utils::OffchainClient<S> for OffchainClient<S> {}

#[derive(Clone, Copy)]
pub struct Node;

impl NodeT for Node {
    type ChainSpec = test_node::ChainSpec;
    type Runtime = Runtime;
    type Block = test_node::OpaqueBlock;

    fn impl_name() -> &'static str {
        test_node::IMPL_NAME
    }

    fn impl_version() -> &'static str {
        test_node::IMPL_VERSION
    }

    fn author() -> &'static str {
        test_node::AUTHOR
    }

    fn copyright_start_year() -> i32 {
        test_node::COPYRIGHT_START_YEAR
    }

    fn chain_spec_dev() -> Self::ChainSpec {
        test_node::development_config()
    }

    fn chain_spec_from_json_bytes(
        json: Vec<u8>,
    ) -> Result<Self::ChainSpec, ChainSpecError> {
        Self::ChainSpec::from_json_bytes(json).map_err(ChainSpecError)
    }

    fn new_light(
        config: Configuration,
    ) -> Result<(TaskManager, RpcHandlers, Network<Self>), sc_service::Error> {
        test_node::new_light(config)
    }

    fn new_full(
        config: Configuration,
    ) -> Result<(TaskManager, RpcHandlers, Network<Self>), sc_service::Error> {
        test_node::new_full(config)
    }
}

pub struct UserDevice;

impl KeyType for UserDevice {
    const KEY_TYPE: u8 = 0;
    type Pair = sr25519::Pair;
}

pub type Client = GenericClient<
    Node,
    UserDevice,
    OffchainClient<OffchainStore<Node>>,
>;
