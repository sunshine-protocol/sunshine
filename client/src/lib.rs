mod runtime;
mod srml;

pub use runtime::{Runtime, RuntimeExtra};
pub use srml::*;
pub use substrate_subxt::{balances, system, ExtrinsicSuccess};

use sp_runtime::MultiSignature;

pub type ClientBuilder = substrate_subxt::ClientBuilder<Runtime, MultiSignature, RuntimeExtra>;
pub type Client = substrate_subxt::Client<Runtime, MultiSignature, RuntimeExtra>;
