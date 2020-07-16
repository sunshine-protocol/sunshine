use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Subxt(#[from] substrate_subxt::Error),
    #[error("{0}")]
    Ipfs(#[from] ipfs_embed::Error),
    #[error(transparent)]
    Keystore(#[from] keybase_keystore::Error),
    #[error(transparent)]
    Octocrab(#[from] octocrab::Error),
    #[error(transparent)]
    Client(#[from] test_client::Error),
    #[error("Configuration directory not found.")]
    ConfigDirNotFound,
    #[error("keystore already initialized")]
    KeystoreInitialized,
    #[error("event not found")]
    EventNotFound,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
}

pub type Result<T> = core::result::Result<T, Error>;
