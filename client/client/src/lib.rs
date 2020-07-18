mod error;
// export client error type for ../cli
pub use error::Error;
pub mod bank;
pub mod bounty;
pub mod court;
pub mod donate;
pub mod org;
pub mod vote;

use codec::{
    Decode,
    Encode,
};
use ipld_block_builder::{
    Cache,
    Codec,
};
use libipld::{
    cbor::DagCborCodec,
    codec::{
        Decode as DagDecode,
        Encode as DagEncode,
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

#[derive(Default, Clone, DagCbor, Encode, Decode)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Default, Clone, DagCbor, Encode, Decode)]
pub struct BountyBody {
    pub repo_owner: String,
    pub repo_name: String,
    pub issue_number: u64,
}

pub(crate) async fn post<R, C, V>(
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
    V: Clone + DagEncode<DagCborCodec> + DagDecode<DagCborCodec> + Send + Sync,
{
    let cid = client.offchain_client().insert(value).await?;
    client.offchain_client().flush().await?;
    let ret_cid = R::IpfsReference::from(cid);
    Ok(ret_cid)
}
