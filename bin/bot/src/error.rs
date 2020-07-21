use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Bounty(#[from] sunshine_bounty_cli::Error<test_client::Error>),
    #[error(transparent)]
    CiDecode(#[from] libipld::cid::Error),
    #[error(transparent)]
    Client(#[from] test_client::Error),
    #[error(transparent)]
    GithuBot(#[from] gbot::Error),
    #[error(transparent)]
    Identity(#[from] sunshine_identity_cli::Error<test_client::Error>),
    #[error(transparent)]
    Libipld(#[from] libipld::error::Error),
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error(transparent)]
    SubxtCodec(#[from] substrate_subxt::sp_runtime::codec::Error),
    #[error("Exit command invoked from CLI")]
    ExitBot,
}

pub type Result<T> = core::result::Result<T, Error>;
