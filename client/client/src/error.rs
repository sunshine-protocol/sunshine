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
    #[error(transparent)]
    Keystore(#[from] keystore::Error),
    #[error("keystore already initialized")]
    KeystoreInitialized,
    #[error("event not found")]
    EventNotFound,
    #[error("Account ID cannot be parsed from string.")]
    AccountIdParseFail,
    #[error("Account ID cannot be parsed from string.")]
    Unsigned64BitIntegerConversionFails,
}

pub type Result<T> = core::result::Result<T, Error>;
