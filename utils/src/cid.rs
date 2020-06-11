use codec::{Decode, Encode};
#[cfg(feature = "std")]
use core::convert::TryFrom;
#[cfg(feature = "std")]
use libipld::cid::{Cid, Error};
#[cfg(feature = "std")]
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
#[cfg(feature = "std")]
use std::fmt;

pub const CID_LENGTH: usize = 38;

#[derive(Clone, Decode, Encode)]
pub struct CidBytes([u8; CID_LENGTH]);

#[cfg(feature = "std")]
impl CidBytes {
    pub fn to_cid(&self) -> Result<Cid, Error> {
        Cid::try_from(&self.0[..])
    }
}

#[cfg(feature = "std")]
impl Serialize for CidBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

#[cfg(feature = "std")]
impl<'de> Deserialize<'de> for CidBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        pub struct CidVisitor;

        impl<'de> Visitor<'de> for CidVisitor {
            type Value = CidBytes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("byte vector of length 38usize")
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() == 38usize {
                    let mut buf = [0; CID_LENGTH];
                    buf.copy_from_slice(&value[..]);
                    Ok(CidBytes(buf))
                } else {
                    Err(E::custom(format!(
                        "byte vector len is {} dne 38",
                        value.len()
                    )))
                }
            }
        }
        deserializer.deserialize_bytes(CidVisitor)
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
