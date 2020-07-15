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
    #[error(transparent)]
    Ipfsembed(#[from] ipfs_embed::Error),

    #[error("Failed to find config dir. Use `--path` to supply a suitable directory.")]
    ConfigDirNotFound,
    #[error(transparent)]
    InvalidSuri(#[from] bounty_client::InvalidSuri),
    #[error("Failed to decode transfer event.")]
    TransferEventDecode,
    #[error("Failed to find transfer event.")]
    TransferEventFind,
    #[error("Device key is already configured. Use `--force` if you want to overwrite it.")]
    HasDeviceKey,
    #[error("Invalid paperkey.")]
    InvalidMnemonic,
    #[error("Password too short.")]
    PasswordTooShort,
    #[error("Passwords don't match.")]
    PasswordMismatch,
    #[error("Vote percent threshold input must be 0 < x < 100")]
    VotePercentThresholdInputBoundError,
    #[error("Input error for posting bounty.")]
    PostBountyInputError,
}

pub type Result<T> = core::result::Result<T, Error>;
