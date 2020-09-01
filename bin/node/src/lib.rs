use sc_executor::native_executor_instance;
use sc_service::ChainType;
use sp_core::{
    sr25519,
    Pair,
    Public,
};
use sp_runtime::traits::{
    IdentifyAccount,
    Verify,
};
use sunshine_node_utils::node_service;
use test_runtime::{
    AccountId,
    AuraConfig,
    Balance,
    BalancesConfig,
    BlockNumber,
    GenesisConfig,
    GrandpaConfig,
    GrantConfig,
    OrgConfig,
    Signature,
    SystemConfig,
    TreasuryConfig,
    WASM_BINARY,
};

pub const IMPL_NAME: &str = "Sunshine Node";
pub const IMPL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const SUPPORT_URL: &str = env!("CARGO_PKG_HOMEPAGE");
pub const COPYRIGHT_START_YEAR: i32 = 2020;
pub const EXECUTABLE_NAME: &str = env!("CARGO_PKG_NAME");

native_executor_instance!(
    pub Executor,
    test_runtime::api::dispatch,
    test_runtime::native_version,
);

node_service!(
    test_runtime::opaque::Block,
    test_runtime::RuntimeApi,
    Executor
);

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(
    seed: &str,
) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate an authority key for Aura
pub fn get_authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        || {
            testnet_genesis(
                // initial authorities
                vec![get_authority_keys_from_seed("Alice")],
                // root key
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // endowed accounts
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                ],
                // first org value constitution
                sunshine_codec::Cid::default(),
                // flat share membership
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                ],
                (10, 10),
                true,
            )
        },
        vec![],
        None,
        None,
        None,
        None,
    )
}

pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        || {
            testnet_genesis(
                // initial authorities
                vec![
                    get_authority_keys_from_seed("Alice"),
                    get_authority_keys_from_seed("Bob"),
                ],
                // root key
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                // endowed accounts
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                    get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
                    get_account_id_from_seed::<sr25519::Public>(
                        "Charlie//stash",
                    ),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>(
                        "Ferdie//stash",
                    ),
                ],
                // first org value constitution
                sunshine_codec::Cid::default(),
                // first org flat membership
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                ],
                (10, 10),
                true,
            )
        },
        vec![],
        None,
        None,
        None,
        None,
    )
}

pub fn testnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    first_org_value_constitution: sunshine_codec::Cid,
    first_org_flat_membership: Vec<AccountId>,
    treasury_mint_rate: (BlockNumber, Balance),
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        grant: Some(GrantConfig {
            application_poll_frequency: 10,
            milestone_poll_frequency: 10,
        }),
        org: Some(OrgConfig {
            sudo: root_key,
            doc: first_org_value_constitution,
            mems: first_org_flat_membership,
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.0.clone()))
                .collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        treasury: Some(TreasuryConfig {
            minting_interval: treasury_mint_rate.0,
            mint_amount: treasury_mint_rate.1,
        }),
    }
}
