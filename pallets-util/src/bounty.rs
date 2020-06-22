use codec::{Decode, Encode};
use sp_core::TypeId;
use sp_std::prelude::*;

// same flow as other bounty module but as simple as possible this time
pub struct PlaceHolder {
    f: u32,
}
