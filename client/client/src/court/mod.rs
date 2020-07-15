mod subxt;

pub use subxt::*;

use crate::error::Error;
use async_trait::async_trait;
use substrate_subxt::{
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[async_trait]
pub trait CourtClient<T: Runtime + Court>: ChainClient<T> {}

#[async_trait]
impl<T, C> CourtClient<T> for C
where
    T: Runtime + Court,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::Error: From<Error>,
{
}
