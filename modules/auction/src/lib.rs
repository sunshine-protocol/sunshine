use support::{
	StorageValue, StorageMap, decl_module, decl_storage, decl_event, storage::child, ensure,
	traits::{Currency, Get, OnUnbalanced, WithdrawReason, ExistenceRequirement}
};
use system::ensure_signed;
use runtime_primitives::{ModuleId, weights::SimpleDispatchInfo,
	traits::{AccountIdConversion, Hash, Saturating, Zero, CheckedAdd}
};
use parity_scale_codec::{Encode, Decode};

// not compiling, no worries :o)
// use substrate_primitives::storage::well_known_keys::CHILD_STORAGE_KEY_PREFIX;

const MODULE_ID: ModuleId = ModuleId(*b"auctions");

