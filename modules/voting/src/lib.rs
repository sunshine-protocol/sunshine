use support::{
	StorageValue, StorageMap, decl_module, decl_storage, decl_event, storage::child, ensure,
	traits::{Currency, Get, OnUnbalanced, WithdrawReason, ExistenceRequirement}
};
use system::ensure_signed;
use runtime_primitives::{ModuleId, weights::SimpleDispatchInfo,
	traits::{AccountIdConversion, Hash, Saturating, Zero, CheckedAdd}
};
use parity_scale_codec::{Encode, Decode};

// I want a generic election module that doesn't necessarily require `Balance` as collateral
// define collateral as a type according to 

pub trait Trait: system::Trait {
	type Collateral: SimpleArithmetic + Codec + Copy + MaybeSerializeDebug + Default;


}
// an improved implementation of this uses a trait for the runtime to define how the collateral is bonded and unbonded
// it might have a `BondState` which is stateful in the runtime and tracks how the bond changes in value while it's bonded
// how much can be purchased back at any time
// extra feature (TODO: add market for buying exit priority `=>` innovate on the dilution mechanism)