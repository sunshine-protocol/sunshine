#![recursion_limit = "256"]

#[macro_use]
extern crate substrate_subxt;

#[macro_use]
extern crate derive_new;

mod error;
#[cfg(feature = "light-client")]
mod light_client;
mod runtime;
mod srml;
mod sunshine;

pub use error::Error;
#[cfg(feature = "light-client")]
pub use light_client::ChainType;
pub use runtime::{Extra, Runtime};
pub use srml::org::Org;
pub use sunshine::SunClient;
