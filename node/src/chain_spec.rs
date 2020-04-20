use sc_service::ChainType;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{sr25519, Pair, Public};
use sp_finality_grandpa::AuthorityId as GrandpaId;
use sp_runtime::traits::{IdentifyAccount, Verify};
use suntime::{
    AccountId,
    AuraConfig,
    BalancesConfig,
    GenesisConfig,
    GrandpaConfig,
    IndicesConfig,
    OrgId,
    Share,
    ShareId,
    SharesAtomicConfig,
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

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed(seed: &str) -> AccountId {
    <Signature as Verify>::Signer::from(get_from_seed::<sr25519::Public>(seed)).into_account()
}

/// Helper function to generate an authority key for Aura
pub fn get_authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

pub struct Org {
    pub org_id: OrgId,
    pub share_id: ShareId,
    pub members: Vec<(AccountId, Share)>,
}

impl Org {
    pub fn membership_shares<'a>(
        &'a self,
    ) -> impl Iterator<Item = (OrgId, ShareId, AccountId, Share)> + 'a {
        self.members
            .iter()
            .map(move |(account, share)| (self.org_id, self.share_id, account.clone(), *share))
    }

    pub fn shareholders(&self) -> (OrgId, ShareId, Vec<AccountId>) {
        (
            self.org_id,
            self.share_id,
            self.members
                .iter()
                .map(|(account, _)| account.clone())
                .collect(),
        )
    }

    pub fn total_issuance(&self) -> (OrgId, ShareId, Share) {
        let mut total = 0;
        for (_, share) in &self.members {
            total += share;
        }
        (self.org_id, self.share_id, total)
    }
}

pub fn development_config() -> ChainSpec {
    let initial_authorities = ["Alice"];
    let root_key = "Alice";
    let accounts = ["Alice", "Bob", "Charlie", "Dave", "Eve", "Fredie"];
    let orgs = vec![Org {
        org_id: 1,
        share_id: 1,
        members: accounts
            .iter()
            .map(|account| (get_account_id_from_seed(account), 10))
            .collect(),
    }];
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        move || testnet_genesis(&initial_authorities, root_key, &accounts, &orgs),
        vec![],
        None,
        None,
        None,
        None,
    )
}

pub fn local_testnet_config() -> ChainSpec {
    let initial_authorities = ["Alice", "Bob"];
    let root_key = "Alice";
    let accounts = ["Alice", "Bob", "Charlie", "Dave", "Eve", "Fredie"];
    let orgs = vec![Org {
        org_id: 1,
        share_id: 1,
        members: accounts
            .iter()
            .map(|account| (get_account_id_from_seed(account), 10))
            .collect(),
    }];
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        move || testnet_genesis(&initial_authorities, root_key, &accounts, &orgs),
        vec![],
        None,
        None,
        None,
        None,
    )
}

pub fn testnet_genesis(
    initial_authorities: &[&str],
    root_key: &str,
    accounts: &[&str],
    orgs: &[Org],
) -> GenesisConfig {
    let initial_authorities = initial_authorities
        .into_iter()
        .map(|authority| get_authority_keys_from_seed(*authority))
        .collect();
    let root_key = get_account_id_from_seed(root_key);
    let stash_accounts = accounts
        .iter()
        .map(|account| format!("{}//stash", account))
        .collect::<Vec<_>>();
    let endowed_accounts = accounts
        .iter()
        .map(|s| *s)
        .chain(stash_accounts.iter().map(|s| &**s))
        .map(get_account_id_from_seed)
        .collect();
    let mut membership_shares = vec![];
    for org in orgs {
        for membership_share in org.membership_shares() {
            membership_shares.push(membership_share);
        }
    }
    let total_issuance = orgs.iter().map(|org| org.total_issuance()).collect();
    let shareholder_membership = orgs.iter().map(|org| org.shareholders()).collect();
    testnet_genesis_config_builder(
        initial_authorities,
        root_key,
        endowed_accounts,
        membership_shares,
        total_issuance,
        shareholder_membership,
    )
}

pub fn testnet_genesis_config_builder(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<AccountId>,
    membership_shares: Vec<(OrgId, ShareId, AccountId, Share)>,
    total_issuance: Vec<(OrgId, ShareId, Share)>,
    shareholder_membership: Vec<(OrgId, ShareId, Vec<AccountId>)>,
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
        shares_atomic: Some(SharesAtomicConfig {
            omnipotent_key: root_key.clone(),
            membership_shares,
            total_issuance,
            shareholder_membership,
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
