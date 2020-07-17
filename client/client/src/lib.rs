mod error;

pub mod bank;
pub mod bounty;
pub mod court;
pub mod donate;
pub mod org;
pub mod vote;

// ipfs ops
pub mod client;

// re-export for usage by cli
pub use ipld_block_builder::{
    Cache,
    Codec,
};
pub use substrate_subxt::sp_runtime::Permill;

pub use error::Error;
