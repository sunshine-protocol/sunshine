use ffi_utils::async_std;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<E: std::error::Error + std::fmt::Debug + 'static> {
    #[error(transparent)]
    Client(E),
    #[error(transparent)]
    Io(#[from] async_std::io::Error),
    #[error(transparent)]
    InvalidSuri(#[from] sunshine_core::InvalidSuri),
    #[error(transparent)]
    InvalidSs58(#[from] sunshine_core::InvalidSs58),
    #[error(transparent)]
    CiDecode(#[from] libipld::cid::Error),
    #[error(transparent)]
    Libipld(#[from] libipld::error::Error),
    #[error(transparent)]
    Cbor(#[from] serde_cbor::Error),
}

pub type Result<T, E> = core::result::Result<T, Error<E>>;
