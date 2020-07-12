use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Subxt(#[from] substrate_subxt::Error),
    #[cfg(feature = "light-client")]
    #[error("{0}")]
    Service(#[from] substrate_subxt_light_client::ServiceError),
    #[error("event not found")]
    EventNotFound,
    #[error("Account ID cannot be parsed from string.")]
    AccountIdParseFail,
    #[error("Number cannot be parsed from string")]
    ParseIntError,
}

pub type Result<T> = core::result::Result<T, Error>;
