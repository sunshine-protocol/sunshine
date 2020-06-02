use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct ShareGroup<OrgId, ShareId> {
    pub org: OrgId,
    pub share: ShareId,
}
