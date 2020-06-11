use codec::{Decode, Encode};

#[derive(Clone, Debug, Eq, PartialEq, Decode, Encode)]
pub struct DeviceMaskData<M, G> {
    pub mask: M,
    pub gen: G,
}
