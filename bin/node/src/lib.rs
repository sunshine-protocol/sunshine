#![allow(clippy::too_many_arguments)]
#![allow(clippy::clone_double_ref)]
mod chain_spec;
#[macro_use]
mod service;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;
#[cfg(feature = "cli")]
pub use command::run as run_cli;

pub use chain_spec::ChainSpec;
pub use service::{
    new_full,
    new_light,
};

pub const IMPL_NAME: &str = "Sunshine Node";
pub const IMPL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const SUPPORT_URL: &str = env!("CARGO_PKG_HOMEPAGE");
pub const COPYRIGHT_START_YEAR: i32 = 2020;
pub const EXECUTABLE_NAME: &str = env!("CARGO_PKG_NAME");

pub enum ChainType {
    Development,
    Local,
}

impl ChainType {
    pub fn chain_spec(&self) -> ChainSpec {
        match self {
            Self::Development => chain_spec::development_config(),
            Self::Local => chain_spec::local_testnet_config(),
        }
    }
}
