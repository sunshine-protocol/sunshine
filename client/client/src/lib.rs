mod error;
// export client error type for ../cli
pub use error::Error;
pub mod bank;
pub mod bounty;
pub mod court;
pub mod donate;
pub mod org;
pub mod vote;

use ipld_block_builder::{
    Cache,
    Codec,
};
use libipld::{
    cbor::DagCborCodec,
    codec::{
        Decode,
        Encode,
    },
    DagCbor,
};
use org::Org;
use substrate_subxt::{
    sp_runtime::traits::SignedExtension,
    Runtime,
    SignedExtra,
};
use sunshine_core::ChainClient;

#[derive(Clone, DagCbor)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Clone, DagCbor)]
pub struct BountyBody {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
}

pub async fn post_text<R, C, V>(
    client: &C,
    value: V,
) -> Result<R::IpfsReference, C::Error>
where
    R: Runtime + Org,
    <R as Org>::IpfsReference: From<libipld::cid::Cid>,
    <<R::Extra as SignedExtra<R>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<R>,
    C::OffchainClient: Cache<Codec, V>,
    C::Error: From<Error>,
    V: Clone + Encode<DagCborCodec> + Decode<DagCborCodec> + Send + Sync,
{
    let cid = client.offchain_client().insert(value).await?;
    let ret_cid = R::IpfsReference::from(cid);
    Ok(ret_cid)
}

pub async fn post_bounty<R, C>(
    client: &C,
    bounty: BountyBody,
) -> Result<R::IpfsReference, C::Error>
where
    R: Runtime + Org,
    <R as Org>::IpfsReference: From<libipld::cid::Cid>,
    <<R::Extra as SignedExtra<R>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: ChainClient<R>,
    C::OffchainClient: Cache<Codec, BountyBody>,
    C::Error: From<Error>,
{
    let cid = client.offchain_client().insert(bounty).await?;
    let ret_cid = R::IpfsReference::from(cid);
    Ok(ret_cid)
}
