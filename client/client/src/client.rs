use crate::error::Error;
use crate::codec::Text;
use crate::org::Org;
use ipld_block_builder::{Cache, Codec};
use substrate_subxt::{Runtime, SignedExtra};
use substrate_subxt::sp_runtime::traits::SignedExtension;
use sunshine_core::ChainClient;

async fn post_ipfs_reference<T, C>(client: &C, text: Text) -> Result<T::IpfsReference, C::Error>
where
    T: Runtime + Org, // TODO? <T as Org>::IpfsReference: From<CidBytes>
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned: Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, Text>,
    C::Error: From<Error>,
{
    let cid = client.offchain_client().insert(text).await?;
    let ret_cid = T::IpfsReference::from(cid);
    Ok(ret_cid)
}

async fn resolve_ipfs_reference<T, C>(client: &C, cid: T::IpfsReference) -> Result<(), C::Error>
where
    T: Runtime + Org,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned: Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, Text>,
    C::Error: From<Error>,
{
    let _ = client.offchain_client().get(cid).await?;
    Ok(())
}