use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Bounty(#[from] bounty_client::Error),
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error(transparent)]
    Io(#[from] async_std::io::Error),
    #[error(transparent)]
    Keystore(#[from] keystore::Error),

    #[error("Failed to find config dir. Use `--path` to supply a suitable directory.")]
    ConfigDirNotFound,
    #[error("Failed to decode transfer event.")]
    TransferEventDecode,
    #[error("Failed to find transfer event.")]
    TransferEventFind,
    #[error("Failed to find an account associated with key.")]
    NoAccount,
    #[error("Device key is already configured. Use `--force` if you want to overwrite it.")]
    HasDeviceKey,
    #[error("Invalid paperkey.")]
    InvalidMnemonic,
}

pub type Result<T> = core::result::Result<T, Error>;
