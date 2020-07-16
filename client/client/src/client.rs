use crate::{
    error::Error,
    org::Org,
};
use ipld_block_builder::{
    Cache,
    Codec,
};
use libipld::DagCbor;
use substrate_subxt::{
    sp_runtime::traits::SignedExtension,
    Runtime,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[derive(Clone, DagCbor)]
pub struct OrgConstitution {
    pub text: String,
}

#[derive(Clone, DagCbor)]
pub struct BountyBody {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
}

pub async fn post_constitution<T, C>(
    client: &C,
    constitution: OrgConstitution,
) -> Result<T::IpfsReference, C::Error>
where
    T: Runtime + Org,
    <T as Org>::IpfsReference: From<
        libipld::cid::CidGeneric<libipld::cid::Codec, libipld::multihash::Code>,
    >,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, OrgConstitution>,
    C::Error: From<Error>,
{
    let cid = client.offchain_client().insert(constitution).await?;
    let ret_cid = T::IpfsReference::from(cid);
    Ok(ret_cid)
}

pub async fn post_bounty<T, C>(
    client: &C,
    bounty: BountyBody,
) -> Result<T::IpfsReference, C::Error>
where
    T: Runtime + Org,
    <T as Org>::IpfsReference: From<
        libipld::cid::CidGeneric<libipld::cid::Codec, libipld::multihash::Code>,
    >,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<T>,
    C::OffchainClient: Cache<Codec, BountyBody>,
    C::Error: From<Error>,
{
    let cid = client.offchain_client().insert(bounty).await?;
    let ret_cid = T::IpfsReference::from(cid);
    Ok(ret_cid)
}
