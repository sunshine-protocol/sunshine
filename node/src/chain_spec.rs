use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use suntime::{
    AccountId,
    AuraConfig,
    BalancesConfig,
    BankConfig,
    GenesisConfig,
    GrandpaConfig,
    IndicesConfig,
    MembershipConfig,
    Shares,
    SharesAtomicConfig,
    SharesMembershipConfig,
    Signature,
    SudoConfig,
    SystemConfig,
    WASM_BINARY, // Signal, VoteId
};

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
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
                // org membership
                None,
                // flat share supervisors
                None,
                // flat share membership
                None,
                // weighted share supervisors
                None,
                // weighted share membership
                None,
                // first org value constitution
                b"build cool shit".to_vec(),
                // flat share membership
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                ],
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
                    get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
                ],
                // org membership
                None,
                // flat share supervisors
                None,
                // flat share membership
                None,
                // weighted share supervisors
                None,
                // weighted share membership
                None,
                // first org value constitution
                b"build cool shit".to_vec(),
                // first org flat membership
                vec![
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    get_account_id_from_seed::<sr25519::Public>("Bob"),
                    get_account_id_from_seed::<sr25519::Public>("Charlie"),
                    get_account_id_from_seed::<sr25519::Public>("Dave"),
                    get_account_id_from_seed::<sr25519::Public>("Eve"),
                    get_account_id_from_seed::<sr25519::Public>("Ferdie"),
                ],
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
    org_membership: Option<Vec<(u32, AccountId, bool)>>,
    flat_share_supervisors: Option<Vec<(u32, u32, AccountId)>>,
    flat_share_membership: Option<Vec<(u32, u32, AccountId, bool)>>,
    weighted_share_supervisors: Option<Vec<(u32, u32, AccountId)>>,
    weighted_share_membership: Option<Vec<(u32, u32, AccountId, Shares)>>,
    first_org_value_constitution: Vec<u8>,
    first_org_flat_membership: Vec<AccountId>,
    _enable_println: bool,
) -> GenesisConfig {
    GenesisConfig {
        frame_system: Some(SystemConfig {
            code: WASM_BINARY.to_vec(),
            changes_trie_config: Default::default(),
        }),
        pallet_balances: Some(BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1 << 60))
                .collect(),
        }),
        pallet_indices: Some(IndicesConfig { indices: vec![] }),
        membership: Some(MembershipConfig {
            omnipotent_key: root_key.clone(),
            membership: org_membership,
        }),
        shares_membership: Some(SharesMembershipConfig {
            share_supervisors: flat_share_supervisors,
            shareholder_membership: flat_share_membership,
        }),
        shares_atomic: Some(SharesAtomicConfig {
            share_supervisors: weighted_share_supervisors,
            shareholder_membership: weighted_share_membership,
        }),
        bank: Some(BankConfig {
            first_organization_supervisor: root_key.clone(),
            first_organization_value_constitution: first_org_value_constitution,
            first_organization_flat_membership: first_org_flat_membership,
        }),
        pallet_aura: Some(AuraConfig {
            authorities: initial_authorities.iter().map(|x| (x.0.clone())).collect(),
        }),
        pallet_grandpa: Some(GrandpaConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
        }),
        pallet_sudo: Some(SudoConfig { key: root_key }),
    }
}
