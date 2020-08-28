#![recursion_limit = "256"]
#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::declare_interior_mutable_const)]
#![cfg_attr(not(feature = "std"), no_std)]
//! `util` contains all objects and relationships for all sunshine-bounty modules
//! - see `traits.rs` for behavioral definitions and other files for object impls
#[macro_use]
extern crate derive_new;

pub mod bank;
pub mod bounty;
pub mod court;
pub mod drip;
pub mod grant;
pub mod kickback;
pub mod meta;
pub mod moloch;
pub mod organization;
pub mod rank;
pub mod rfp;
pub mod share;
pub mod sss;
pub mod traits;
pub mod vote;
