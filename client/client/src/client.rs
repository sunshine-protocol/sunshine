use crate::{
    codec::Text,
    error::Error,
    org::Org,
};
use ipld_block_builder::{
    Cache,
    Codec,
    ReadonlyCache,
};
use substrate_subxt::{
    sp_runtime::traits::SignedExtension,
    Runtime,
    SignedExtra,
};
use sunshine_core::ChainClient;

pub async fn post_ipfs_reference<T, C>(
    client: &C,
    text: Text,
) -> Result<T::IpfsReference, C::Error>
where
    T: Runtime + Org,
    <T as Org>::IpfsReference: From<
        libipld::cid::CidGeneric<libipld::cid::Codec, libipld::multihash::Code>,
    >,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, Text>,
    C::Error: From<Error>,
{
    let cid = client.offchain_client().insert(text).await?;
    let ret_cid = T::IpfsReference::from(cid);
    Ok(ret_cid)
}

pub async fn resolve_ipfs_reference<T, C>(
    client: &C,
    cid: T::IpfsReference,
) -> Result<(), C::Error>
where
    T: Runtime + Org,
    <T as Org>::IpfsReference: Into<
        libipld::cid::CidGeneric<libipld::cid::Codec, libipld::multihash::Code>,
    >,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, Text>,
    C::Error: From<Error>,
{
    let _ = client.offchain_client().get(&cid.into()).await?;
    Ok(())
}
