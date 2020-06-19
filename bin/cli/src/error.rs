use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sunshine(#[from] sunshine_client::Error),
    #[error(transparent)]
    Ipfs(#[from] ipfs_embed::Error),
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error(transparent)]
    Io(#[from] async_std::io::Error),
    #[error(transparent)]
    Keystore(#[from] keybase_keystore::Error),
    #[error(transparent)]
    Qr(#[from] qr2term::QrError),

    #[error("Failed to find config dir. Use `--path` to supply a suitable directory.")]
    ConfigDirNotFound,
    #[error("Invalid suri encoded key pair.")]
    InvalidSuri,
    #[error("Invalid ss58 encoded account id.")]
    InvalidSs58,
    #[error("Device key is already configured. Use `--force` if you want to overwrite it.")]
    HasDeviceKey,
    #[error("Password too short.")]
    PasswordTooShort,
    #[error("Invalid paperkey.")]
    InvalidMnemonic,
    #[error("Passed in identifier cannot be made into u64.")]
    IdentifierConversionFailed,
    #[error("Passed in account identifier cannot be formed into AccountId type.")]
    AccountIdConversionFailed,
    #[error("Identifier cannot be parsed.")]
    UnparsedIdentifier,
}
