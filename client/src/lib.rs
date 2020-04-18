mod frame;
mod runtime;

pub use frame::*;
pub use runtime::{Runtime, RuntimeExtra};
pub use substrate_subxt::{balances, system, ExtrinsicSuccess};
use thiserror::Error;

use sp_runtime::MultiSignature;

pub type ClientBuilder = substrate_subxt::ClientBuilder<Runtime, MultiSignature, RuntimeExtra>;
pub type Client = substrate_subxt::Client<Runtime, MultiSignature, RuntimeExtra>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Subxt(#[from] substrate_subxt::Error),
    #[cfg(feature = "light-client")]
    #[error("{0}")]
    Service(#[from] substrate_subxt_light_client::ServiceError),
}

pub async fn build_client() -> Result<Client, Error> {
    let cli = ClientBuilder::new();
    #[cfg(feature = "light-client")]
    let cli = {
        use std::path::PathBuf;
        use substrate_subxt_light_client::{LightClient, LightClientConfig};
        let config = LightClientConfig {
            impl_name: sunshine_node::IMPL_NAME,
            impl_version: sunshine_node::IMPL_VERSION,
            author: sunshine_node::AUTHOR,
            copyright_start_year: sunshine_node::COPYRIGHT_START_YEAR,
            builder: sunshine_node::new_light,
            db_path: PathBuf::from("/tmp/sunshine-light-client"),
            chain_spec: sunshine_node::ChainType::Local.chain_spec(),
        };
        cli.set_client(LightClient::new(config)?)
    };
    Ok(cli.build().await?)
}
