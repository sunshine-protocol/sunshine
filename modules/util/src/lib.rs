#![allow(clippy::type_complexity)]
#![allow(clippy::too_many_arguments)]
#![cfg_attr(not(feature = "std"), no_std)]
//! `util` contains all objects and relationships for all sunshine modules
//! - see `traits.rs` for behavioral definitions and other files for object impls

pub mod bank;
pub mod bounty;
pub mod court;
pub mod organization;
pub mod petition;
pub mod proposal;
pub mod schedule;
pub mod share;
pub mod traits;
pub mod uuid;
pub mod voteyesno;
