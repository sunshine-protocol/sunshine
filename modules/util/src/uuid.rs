use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct UUID(u32);

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct UUID2(u32, u32);

// impl From<(u32, u32)> for UUID2 {
//     fn from(other: (u32, u32)) -> UUID2 {
//         UUID2::new(other.1, other.2)
//     }
// }

impl UUID2 {
    pub fn new(one: u32, two: u32) -> UUID2 {
        UUID2(one, two)
    }
    pub fn one(&self) -> u32 {
        self.0
    }
    pub fn two(&self) -> u32 {
        self.1
    }
}

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct UUID3(u32, u32, u32);

impl UUID3 {
    pub fn new(one: u32, two: u32, three: u32) -> UUID3 {
        UUID3(one, two, three)
    }
    pub fn one_two(&self) -> UUID2 {
        UUID2::new(self.0, self.1)
    }
    pub fn one(&self) -> u32 {
        self.0
    }
    pub fn two(&self) -> u32 {
        self.1
    }
    pub fn three(&self) -> u32 {
        self.2
    }
}

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct UUID4(u32, u32, u32, u32);

impl UUID4 {
    pub fn new(one: u32, two: u32, three: u32, four: u32) -> UUID4 {
        UUID4(one, two, three, four)
    }
    pub fn one_two_three(&self) -> UUID3 {
        UUID3::new(self.0, self.1, self.2)
    }
    pub fn one_two(&self) -> UUID2 {
        UUID2::new(self.0, self.1)
    }
    pub fn one(&self) -> u32 {
        self.0
    }
    pub fn two(&self) -> u32 {
        self.1
    }
    pub fn three(&self) -> u32 {
        self.2
    }
    pub fn four(&self) -> u32 {
        self.3
    }
}

use crate::organization::FormedOrganization;

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
// intended usage is FormedOrg + BountyId prefix for other storage items in `bounty`
pub struct FormedOrgUUID23<T> {
    org: FormedOrganization,
    id: T,
}
