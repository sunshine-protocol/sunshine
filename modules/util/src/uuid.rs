use codec::{Codec, Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct ShareGroup<OrgId: Codec + PartialEq, ShareId: Codec + PartialEq> {
    pub org: OrgId,
    pub share: ShareId,
}
