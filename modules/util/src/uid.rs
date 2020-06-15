use codec::{Codec, Encode, Decode};

#[derive(new, PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct IdWrapper<T: Codec> {
    pub id: T,
}