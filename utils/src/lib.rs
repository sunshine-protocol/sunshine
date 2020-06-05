#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use core::convert::TryFrom;
#[cfg(feature = "std")]
use libipld::cid::{Cid, Error};

pub const CID_LENGTH: usize = 38;

#[derive(Clone, Decode, Encode)]
pub struct CidBytes([u8; CID_LENGTH]);

#[cfg(feature = "std")]
impl CidBytes {
    pub fn to_cid(&self) -> Result<Cid, Error> {
        Cid::try_from(&self.0[..])
    }
}

impl core::fmt::Debug for CidBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{:?}", self.0.to_vec())
    }
}

impl Default for CidBytes {
    fn default() -> Self {
        Self([0; CID_LENGTH])
    }
}

impl PartialEq for CidBytes {
    fn eq(&self, other: &Self) -> bool {
        self.0[..] == other.0[..]
    }
}

impl Eq for CidBytes {}

#[cfg(feature = "std")]
impl<'a> From<&'a Cid> for CidBytes {
    fn from(cid: &'a Cid) -> Self {
        let bytes = cid.to_bytes();
        let mut buf = [0; CID_LENGTH];
        buf.copy_from_slice(&bytes[..]);
        Self(buf)
    }
}

#[cfg(feature = "std")]
impl From<Cid> for CidBytes {
    fn from(cid: Cid) -> Self {
        Self::from(&cid)
    }
}

#[cfg(feature = "std")]
impl TryFrom<CidBytes> for Cid {
    type Error = Error;

    fn try_from(cid: CidBytes) -> Result<Self, Self::Error> {
        cid.to_cid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libipld::cid::Codec;
    use libipld::multihash::Blake2b256;

    #[test]
    fn test_cid_bytes() {
        let content = b"hello world";
        let hash = Blake2b256::digest(&content[..]);
        let cid = Cid::new_v1(Codec::Raw, hash);
        let bytes = CidBytes::from(&cid);
        let cid2 = bytes.to_cid().unwrap();
        assert_eq!(cid, cid2);
    }
}
