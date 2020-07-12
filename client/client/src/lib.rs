#![recursion_limit = "256"]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate substrate_subxt;

mod r#abstract;
mod client;
mod error;
mod srml;
mod utils;

pub use client::Client;
pub use error::Error;
pub use r#abstract::AbstractClient;

pub use srml::{
    bank::Bank,
    bounty::Bounty,
    donate::Donate,
    org::Org,
    vote::Vote,
};
pub use utils::AccountShare;
