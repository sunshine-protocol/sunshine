#![recursion_limit = "256"]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate substrate_subxt;

mod client;
mod error;
#[cfg(feature = "light-client")]
mod light_client;
mod runtime;
mod srml;

pub use client::Client;
pub use error::Error;
#[cfg(feature = "light-client")]
pub use light_client::ChainType;
pub use runtime::Runtime;
pub use srml::org::Org;
