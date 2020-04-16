pub mod chain_spec;
#[macro_use]
mod service;

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "cli")]
mod command;
#[cfg(feature = "cli")]
pub use command::run as run_cli;

#[cfg(feature = "light")]
pub use service::new_light;
