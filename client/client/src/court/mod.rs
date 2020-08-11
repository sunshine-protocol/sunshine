mod subxt;

pub use subxt::*;

use substrate_subxt::{
    Runtime,
    SignedExtension,
    SignedExtra,
};
use sunshine_client_utils::{
    async_trait,
    Client,
};

#[async_trait]
pub trait CourtClient<T: Runtime + Court>: Client<T> {}

#[async_trait]
impl<T, C> CourtClient<T> for C
where
    T: Runtime + Court,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<T>,
{
}
