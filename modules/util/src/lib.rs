#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! `util` contains all behavior and relationships for all sunshine modules:
//! - [`shares-atomic`](../shares_atomic/index.html)
//! - [`vote-yesno`](../vote_yesno/index.html)
//! - [`bank`](../bank/index.html)

pub mod bounty;
pub mod court;
pub mod organization;
pub mod proposal;
pub mod schedule;
pub mod share;
pub mod traits;
pub mod uuid;
pub mod voteyesno;
