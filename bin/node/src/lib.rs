#![allow(clippy::too_many_arguments)]
#![allow(clippy::clone_double_ref)]
pub mod chain_spec;
#[macro_use]
pub mod service;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;
#[cfg(feature = "cli")]
pub use command::run as run_cli;

pub const IMPL_NAME: &str = "Sunshine Node";
pub const IMPL_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const SUPPORT_URL: &str = env!("CARGO_PKG_HOMEPAGE");
pub const COPYRIGHT_START_YEAR: i32 = 2020;
pub const EXECUTABLE_NAME: &str = env!("CARGO_PKG_NAME");
