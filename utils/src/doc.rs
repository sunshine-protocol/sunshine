use codec::{
    Decode,
    Encode,
};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Default, Clone, Encode, Decode, RuntimeDebug)]
pub struct FullDoc<Id, Doc> {
    pub id: Id,
    pub doc: Doc,
}