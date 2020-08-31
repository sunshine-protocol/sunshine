mod error;
// export client error type for ../cli
pub use error::Error;
pub mod bank;
pub mod bounty;
pub mod donate;
pub mod org;
pub mod vote;
pub use sunshine_bounty_utils as utils;

use libipld::{
    cache::Cache,
    cbor::DagCborCodec,
    codec::{
        Decode as DagDecode,
        Encode as DagEncode,
    },
    store::ReadonlyStore,
    DagCbor,
};
use parity_scale_codec::{
    Decode,
    Encode,
};
use substrate_subxt::{
    sp_runtime::traits::SignedExtension,
    Runtime,
    SignedExtra,
};
use sunshine_client_utils::{
    Client,
    OffchainClient,
    Result,
};

#[derive(Default, Clone, DagCbor, Encode, Decode)]
pub struct TextBlock {
    pub text: String,
}

#[derive(Debug, Default, Clone, DagCbor, Encode, Decode)]
pub struct GithubIssue {
    pub issue_number: u64,
    pub repo_owner: String,
    pub repo_name: String,
}

pub(crate) async fn post<R, C, V>(
    client: &C,
    value: V,
) -> Result<sunshine_codec::Cid>
where
    R: Runtime,
    <<R::Extra as SignedExtra<R>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    C: Client<R>,
    C::OffchainClient:
        Cache<<C::OffchainClient as OffchainClient>::Store, DagCborCodec, V>,
    <<C::OffchainClient as OffchainClient>::Store as ReadonlyStore>::Codec:
        From<DagCborCodec> + Into<DagCborCodec>,
    V: Clone + DagEncode<DagCborCodec> + DagDecode<DagCborCodec> + Send + Sync,
{
    let cid = client.offchain_client().insert(value).await?;
    client.offchain_client().flush().await?;
    Ok(cid)
}
