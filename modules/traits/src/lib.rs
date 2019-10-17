// like srml-support but for sunshine

use rstd::{prelude::*, result, marker::PhantomData, ops::Div};
use codec::{FullCodec, Codec, Encode, Decode};
use primitives::u32_trait::Value as U32;
use sr_primitives::{
	ConsensusEngineId,
	traits::{MaybeSerializeDebug, SimpleArithmetic, Saturating},
};

/// Trait for a hook to get called when some balance has been minted, causing dilution.
pub trait OnDilution<Balance> {
	/// Some `portion` of the total balance just "grew" by `minted`. `portion` is the pre-growth
	/// amount (it doesn't take account of the recent growth).
	fn on_dilution(minted: Balance, portion: Balance);
}

impl<Balance> OnDilution<Balance> for () {
	fn on_dilution(_minted: Balance, _portion: Balance) {}
}

/// Outcome of a balance update.
pub enum UpdateBalanceOutcome {
	/// Account balance was simply updated.
	Updated,
	/// The update led to killing the account.
	AccountKilled,
}

/// Something which can compute and check proofs of
/// a historical key owner and return full identification data of that
/// key owner.
pub trait KeyOwnerProofSystem<Key> {
	/// The proof of membership itself.
	type Proof: Codec;
	/// The full identification of a key owner and the stash account.
	type IdentificationTuple: Codec;

	/// Prove membership of a key owner in the current block-state.
	///
	/// This should typically only be called off-chain, since it may be
	/// computationally heavy.
	///
	/// Returns `Some` iff the key owner referred to by the given `key` is a
	/// member of the current set.
	fn prove(key: Key) -> Option<Self::Proof>;

	/// Check a proof of membership on-chain. Return `Some` iff the proof is
	/// valid and recent enough to check.
	fn check_proof(key: Key, proof: Self::Proof) -> Option<Self::IdentificationTuple>;
}

/// Handler for when some currency "account" decreased in balance for
/// some reason.
///
/// The only reason at present for an increase would be for validator rewards, but
/// there may be other reasons in the future or for other chains.
///
/// Reasons for decreases include:
///
/// - Someone got slashed.
/// - Someone paid for a transaction to be included.
pub trait OnUnbalanced<Imbalance> {
	/// Handler for some imbalance. Infallible.
	fn on_unbalanced(amount: Imbalance);
}

impl<Imbalance: Drop> OnUnbalanced<Imbalance> for () {
	fn on_unbalanced(amount: Imbalance) { drop(amount); }
}