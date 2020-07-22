use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    CiDecode(#[from] libipld::cid::Error),
    #[error(transparent)]
    Client(#[from] test_client::Error),
    #[error(transparent)]
    GithuBot(#[from] gbot::Error),
    #[error(transparent)]
    Libipld(#[from] libipld::error::Error),
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error(transparent)]
    SubxtCodec(#[from] substrate_subxt::sp_runtime::codec::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
