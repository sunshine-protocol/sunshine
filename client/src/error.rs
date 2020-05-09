use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Subxt(#[from] substrate_subxt::Error),
    #[cfg(feature = "light-client")]
    #[error("{0}")]
    Service(#[from] substrate_subxt_light_client::ServiceError),
    #[error("{0}")]
    Sled(#[from] sled::Error),
    #[error("{0}")]
    Ipfs(#[from] ipfs_embed::Error),
    #[error("event not found")]
    EventNotFound,
}
