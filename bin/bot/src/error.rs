use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Bounty(#[from] sunshine_bounty_cli::Error<test_client::Error>),
    #[error(transparent)]
    Client(#[from] test_client::Error),
    #[error(transparent)]
    GithuBot(#[from] gbot::Error),
    #[error(transparent)]
    Identity(#[from] sunshine_identity_cli::Error<test_client::Error>),
    #[error(transparent)]
    Subxt(#[from] substrate_subxt::Error),
    #[error("Exit command invoked from CLI")]
    ExitBot,
}

pub type Result<T> = core::result::Result<T, Error>;
